use boed::{BayesianModel, DesignOptimizer, ExpectedInformationGain, MonteCarloEstimator};

#[derive(Clone, Debug)]
struct ScalarGaussianModel {
    prior_mean: f64,
    prior_std: f64,
    noise_std: f64,
}

impl BayesianModel for ScalarGaussianModel {
    type Design = f64;
    type Parameter = f64;
    type Observation = f64;

    fn sample_prior(&self, draw_index: usize) -> Self::Parameter {
        const OFFSETS: [f64; 7] = [-1.75, -1.0, -0.25, 0.0, 0.25, 1.0, 1.75];
        self.prior_mean + self.prior_std * OFFSETS[draw_index % OFFSETS.len()]
    }

    fn sample_observation(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        draw_index: usize,
    ) -> Self::Observation {
        const NOISE: [f64; 7] = [-0.3, -0.15, -0.05, 0.0, 0.05, 0.15, 0.3];
        design * parameter + self.noise_std * NOISE[draw_index % NOISE.len()]
    }

    fn log_likelihood(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        observation: &Self::Observation,
    ) -> f64 {
        let residual = observation - design * parameter;
        let variance = self.noise_std * self.noise_std;
        -0.5 * residual * residual / variance
    }
}

#[test]
fn stronger_design_has_higher_information_gain() {
    let model = ScalarGaussianModel {
        prior_mean: 0.0,
        prior_std: 1.0,
        noise_std: 0.25,
    };

    let estimator = MonteCarloEstimator::new(64);
    let optimizer = DesignOptimizer::new(estimator, ExpectedInformationGain);
    let candidates = [0.25, 0.5, 1.0, 2.0];

    let best = optimizer.optimize(&model, &candidates).unwrap();

    assert_eq!(best.design, 2.0);
    assert!(best.expected_utility.is_finite());
}

#[test]
fn estimator_rejects_zero_samples() {
    let model = ScalarGaussianModel {
        prior_mean: 0.0,
        prior_std: 1.0,
        noise_std: 0.25,
    };

    let estimator = MonteCarloEstimator::new(0);
    let err = estimator
        .estimate(&model, &1.0, &ExpectedInformationGain)
        .unwrap_err();

    assert_eq!(err, boed::BoedError::ZeroSamples);
}

#[derive(Clone, Debug)]
struct CountingModel;

impl BayesianModel for CountingModel {
    type Design = ();
    type Parameter = usize;
    type Observation = usize;

    fn sample_prior(&self, draw_index: usize) -> Self::Parameter {
        draw_index
    }

    fn sample_observation(
        &self,
        _design: &Self::Design,
        parameter: &Self::Parameter,
        _draw_index: usize,
    ) -> Self::Observation {
        *parameter
    }

    fn log_likelihood(
        &self,
        _design: &Self::Design,
        parameter: &Self::Parameter,
        observation: &Self::Observation,
    ) -> f64 {
        if parameter == observation {
            0.0
        } else {
            f64::NEG_INFINITY
        }
    }
}

#[test]
fn eig_uses_independent_evidence_samples() {
    let model = CountingModel;
    let estimator = MonteCarloEstimator::new(4);

    let utility = estimator
        .estimate(&model, &(), &ExpectedInformationGain)
        .unwrap();

    assert_eq!(utility, f64::INFINITY);
}

#[derive(Clone, Debug)]
struct StreamSeparationModel;

impl BayesianModel for StreamSeparationModel {
    type Design = ();
    type Parameter = usize;
    type Observation = usize;

    fn sample_prior(&self, draw_index: usize) -> Self::Parameter {
        draw_index
    }

    fn sample_observation(
        &self,
        _design: &Self::Design,
        _parameter: &Self::Parameter,
        draw_index: usize,
    ) -> Self::Observation {
        draw_index
    }

    fn log_likelihood(
        &self,
        _design: &Self::Design,
        _parameter: &Self::Parameter,
        _observation: &Self::Observation,
    ) -> f64 {
        0.0
    }
}

#[test]
fn observation_stream_is_separate_from_prior_stream() {
    let model = StreamSeparationModel;
    let estimator = MonteCarloEstimator::new(3);

    let utility = estimator
        .estimate(&model, &(), &ExpectedInformationGain)
        .unwrap();

    assert_eq!(utility, 0.0);
}
