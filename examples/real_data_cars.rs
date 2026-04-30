use boed::{
    BayesianModel, DesignOptimizer, ExpectedInformationGain, MonteCarloEstimator, PosteriorUpdate,
    SequentialDesignSession,
};
const CARS_CSV: &str = include_str!("data/cars.csv");

#[derive(Clone, Copy, Debug)]
struct BrakeParticle {
    intercept: f64,
    slope: f64,
}

#[derive(Clone, Debug)]
struct CarsBrakeModel {
    particles: Vec<BrakeParticle>,
    residual_bank: Vec<f64>,
    noise_std: f64,
}

impl BayesianModel for CarsBrakeModel {
    type Design = f64;
    type Parameter = BrakeParticle;
    type Observation = f64;

    fn sample_prior(&self, draw_index: usize) -> Self::Parameter {
        self.particles[draw_index % self.particles.len()]
    }

    fn sample_observation(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        draw_index: usize,
    ) -> Self::Observation {
        let residual = self.residual_bank[draw_index % self.residual_bank.len()];
        parameter.intercept + parameter.slope * *design + residual
    }

    fn log_likelihood(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        observation: &Self::Observation,
    ) -> f64 {
        let mean = parameter.intercept + parameter.slope * *design;
        let residual = observation - mean;
        let variance = self.noise_std * self.noise_std;
        -0.5 * residual * residual / variance
    }
}

impl PosteriorUpdate for CarsBrakeModel {
    fn posterior_update(&self, design: &Self::Design, observation: &Self::Observation) -> Self {
        let log_weights = self
            .particles
            .iter()
            .map(|particle| self.log_likelihood(design, particle, observation))
            .collect::<Vec<_>>();

        let max_log_weight = log_weights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        let weights = if max_log_weight == f64::NEG_INFINITY {
            vec![1.0 / self.particles.len() as f64; self.particles.len()]
        } else {
            let unnormalized = log_weights
                .iter()
                .map(|value| (value - max_log_weight).exp())
                .collect::<Vec<_>>();
            let total = unnormalized.iter().sum::<f64>();
            unnormalized
                .into_iter()
                .map(|value| value / total)
                .collect()
        };

        Self {
            particles: systematic_resample(&self.particles, &weights),
            residual_bank: self.residual_bank.clone(),
            noise_std: self.noise_std,
        }
    }
}

fn main() {
    let cars_data = load_cars_data();
    let fitted = fit_linear_model(&cars_data);
    let particles = build_prior_particles(&fitted);
    let model = CarsBrakeModel {
        particles,
        residual_bank: fitted.residuals.clone(),
        noise_std: fitted.noise_std,
    };

    let estimator = MonteCarloEstimator::new(64);
    let optimizer = DesignOptimizer::new(estimator, ExpectedInformationGain);
    let mut session = SequentialDesignSession::new(model);
    let candidates = [8.0, 12.0, 16.0, 20.0, 24.0];

    println!("Real-data BOED example: braking distance calibration");
    println!("Dataset: R `cars` (speed in mph, stopping distance in ft)");
    println!("Loaded from: examples/data/cars.csv");
    println!(
        "Baseline fit: dist ~= {:.2} + {:.2} * speed, noise std ~= {:.2}",
        fitted.intercept, fitted.slope, fitted.noise_std
    );
    println!(
        "Initial posterior mean: intercept {:.2}, slope {:.2}",
        mean_intercept(session.model()),
        mean_slope(session.model())
    );

    let first = optimizer.optimize(session.model(), &candidates).unwrap();
    println!(
        "Step 1 proposal: test speed {:.0} mph (expected utility {:.3})",
        first.design, first.expected_utility
    );

    let observed = retrospective_observation(&cars_data, first.design);
    println!(
        "Observed real-data stopping distance at {:.0} mph: {:.1} ft",
        first.design, observed
    );
    session.update(&first.design, &observed);

    println!(
        "Updated posterior mean: intercept {:.2}, slope {:.2}",
        mean_intercept(session.model()),
        mean_slope(session.model())
    );

    let second = optimizer.optimize(session.model(), &candidates).unwrap();
    println!(
        "Step 2 proposal after update: test speed {:.0} mph (expected utility {:.3})",
        second.design, second.expected_utility
    );
}

#[derive(Clone, Debug)]
struct FittedLine {
    intercept: f64,
    slope: f64,
    noise_std: f64,
    residuals: Vec<f64>,
    mean_x: f64,
    sxx: f64,
}

fn load_cars_data() -> Vec<(f64, f64)> {
    CARS_CSV
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut values = line.split(',');
            let speed = values
                .next()
                .expect("cars.csv row missing speed")
                .parse::<f64>()
                .expect("cars.csv speed is not numeric");
            let distance = values
                .next()
                .expect("cars.csv row missing dist")
                .parse::<f64>()
                .expect("cars.csv dist is not numeric");
            (speed, distance)
        })
        .collect()
}

fn fit_linear_model(data: &[(f64, f64)]) -> FittedLine {
    let n = data.len() as f64;
    let mean_x = data.iter().map(|(x, _)| *x).sum::<f64>() / n;
    let mean_y = data.iter().map(|(_, y)| *y).sum::<f64>() / n;

    let sxx = data.iter().map(|(x, _)| (x - mean_x).powi(2)).sum::<f64>();
    let sxy = data
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();

    let slope = sxy / sxx;
    let intercept = mean_y - slope * mean_x;
    let residuals = data
        .iter()
        .map(|(x, y)| y - (intercept + slope * x))
        .collect::<Vec<_>>();
    let rss = residuals
        .iter()
        .map(|residual| residual * residual)
        .sum::<f64>();
    let noise_std = (rss / (n - 2.0)).sqrt();

    FittedLine {
        intercept,
        slope,
        noise_std,
        residuals,
        mean_x,
        sxx,
    }
}

fn build_prior_particles(fitted: &FittedLine) -> Vec<BrakeParticle> {
    let n = fitted.residuals.len() as f64;
    let slope_se = fitted.noise_std / fitted.sxx.sqrt();
    let intercept_se =
        fitted.noise_std * (1.0 / n + fitted.mean_x * fitted.mean_x / fitted.sxx).sqrt();
    let offsets = [-1.5, -0.75, 0.0, 0.75, 1.5];

    let mut particles = Vec::new();
    for intercept_offset in offsets {
        for slope_offset in offsets {
            particles.push(BrakeParticle {
                intercept: fitted.intercept + intercept_offset * intercept_se,
                slope: fitted.slope + slope_offset * slope_se,
            });
        }
    }

    particles
}

fn systematic_resample(particles: &[BrakeParticle], weights: &[f64]) -> Vec<BrakeParticle> {
    let n = particles.len();
    let mut cumulative = Vec::with_capacity(n);
    let mut running_total = 0.0;
    for weight in weights {
        running_total += *weight;
        cumulative.push(running_total);
    }

    let mut resampled = Vec::with_capacity(n);
    let step = 1.0 / n as f64;
    let mut particle_index = 0usize;

    for draw in 0..n {
        let target = (draw as f64 + 0.5) * step;
        while particle_index + 1 < n && cumulative[particle_index] < target {
            particle_index += 1;
        }
        resampled.push(particles[particle_index]);
    }

    resampled
}

fn retrospective_observation(data: &[(f64, f64)], speed: f64) -> f64 {
    let mut observations = data
        .iter()
        .filter(|(candidate_speed, _)| *candidate_speed == speed)
        .map(|(_, distance)| *distance)
        .collect::<Vec<_>>();
    observations.sort_by(f64::total_cmp);
    observations[observations.len() / 2]
}

fn mean_intercept(model: &CarsBrakeModel) -> f64 {
    model
        .particles
        .iter()
        .map(|particle| particle.intercept)
        .sum::<f64>()
        / model.particles.len() as f64
}

fn mean_slope(model: &CarsBrakeModel) -> f64 {
    model
        .particles
        .iter()
        .map(|particle| particle.slope)
        .sum::<f64>()
        / model.particles.len() as f64
}
