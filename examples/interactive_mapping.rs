use boed::{
    BayesianModel, ContinuousDesignOptimizer, ContinuousDesignSpace, ExpectedInformationGain,
    MonteCarloEstimator, PosteriorUpdate, SequentialDesignSession,
};

#[derive(Clone, Debug)]
struct BoundaryMappingModel {
    support: Vec<f64>,
}

impl BoundaryMappingModel {
    fn new(support: &[f64]) -> Self {
        Self {
            support: support.to_vec(),
        }
    }

    fn observe(&self, design: f64, true_boundary: f64) -> bool {
        design <= true_boundary
    }
}

impl BayesianModel for BoundaryMappingModel {
    type Design = f64;
    type Parameter = f64;
    type Observation = bool;

    fn sample_prior(&self, draw_index: usize) -> Self::Parameter {
        self.support[draw_index % self.support.len()]
    }

    fn sample_observation(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        _draw_index: usize,
    ) -> Self::Observation {
        *design <= *parameter
    }

    fn log_likelihood(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        observation: &Self::Observation,
    ) -> f64 {
        if (*design <= *parameter) == *observation {
            0.0
        } else {
            f64::NEG_INFINITY
        }
    }
}

impl PosteriorUpdate for BoundaryMappingModel {
    fn posterior_update(&self, design: &Self::Design, observation: &Self::Observation) -> Self {
        let support = self
            .support
            .iter()
            .copied()
            .filter(|location| (*design <= *location) == *observation)
            .collect::<Vec<_>>();

        Self::new(&support)
    }
}

fn main() {
    let optimizer =
        ContinuousDesignOptimizer::new(MonteCarloEstimator::new(128), ExpectedInformationGain)
            .with_grid_size(9)
            .with_refinement_rounds(5);

    let space = ContinuousDesignSpace::new(0.0, 1.0).unwrap();
    let mut session =
        SequentialDesignSession::new(BoundaryMappingModel::new(&[0.2, 0.4, 0.6, 0.8]));
    let true_boundary = 0.8;

    for step in 0..2 {
        let best = session.choose_continuous(&optimizer, space).unwrap();
        let observation = session.model().observe(best.design, true_boundary);
        println!(
            "step {}: probe at {:.3}, expected utility {:.3}, observed {}",
            step + 1,
            best.design,
            best.expected_utility,
            observation
        );
        session.update(&best.design, &observation);
    }
}
