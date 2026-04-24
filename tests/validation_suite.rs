use boed::{
    BayesianModel, ContinuousDesignOptimizer, ContinuousDesignSpace, DesignOptimizer,
    ExpectedInformationGain, MonteCarloEstimator, PosteriorUpdate, SequentialDesignSession,
    UtilityFunction,
};

#[derive(Clone, Debug)]
struct SplitMappingModel {
    support: Vec<f64>,
}

impl SplitMappingModel {
    fn new(support: &[f64]) -> Self {
        Self {
            support: support.to_vec(),
        }
    }
}

impl BayesianModel for SplitMappingModel {
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

impl PosteriorUpdate for SplitMappingModel {
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

#[derive(Clone, Debug)]
struct DummyModel;

impl BayesianModel for DummyModel {
    type Design = f64;
    type Parameter = ();
    type Observation = ();

    fn sample_prior(&self, _draw_index: usize) -> Self::Parameter {}

    fn sample_observation(
        &self,
        _design: &Self::Design,
        _parameter: &Self::Parameter,
        _draw_index: usize,
    ) -> Self::Observation {
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

#[derive(Clone, Copy, Debug)]
struct QuadraticUtility {
    optimum: f64,
}

impl UtilityFunction<DummyModel> for QuadraticUtility {
    fn utility(
        &self,
        _model: &DummyModel,
        design: &f64,
        _parameter: &(),
        _observation: &(),
        _evidence_parameters: &[()],
    ) -> f64 {
        -(*design - self.optimum).powi(2)
    }
}

fn exact_threshold_eig(support: &[f64], design: f64) -> f64 {
    let prior = 1.0 / support.len() as f64;
    let p_true = support.iter().filter(|theta| design <= **theta).count() as f64 * prior;
    let p_false = 1.0 - p_true;

    [p_false, p_true]
        .into_iter()
        .filter(|probability| *probability > 0.0)
        .map(|probability| -probability * probability.ln())
        .sum()
}

#[test]
fn finite_optimizer_matches_exact_discrete_threshold_benchmark() {
    let model = SplitMappingModel::new(&[0.2, 0.4, 0.6, 0.8]);
    let estimator = MonteCarloEstimator::new(128);
    let optimizer = DesignOptimizer::new(estimator, ExpectedInformationGain);
    let candidates = [0.1, 0.3, 0.5, 0.7, 0.9];

    let estimated_best = optimizer.optimize(&model, &candidates).unwrap();
    let exact_best = candidates
        .iter()
        .copied()
        .max_by(|left, right| {
            exact_threshold_eig(&model.support, *left)
                .partial_cmp(&exact_threshold_eig(&model.support, *right))
                .unwrap()
        })
        .unwrap();

    assert_eq!(estimated_best.design, exact_best);
    assert_eq!(estimated_best.design, 0.5);
}

#[test]
fn continuous_optimizer_finds_known_quadratic_peak() {
    let optimizer = ContinuousDesignOptimizer::new(
        MonteCarloEstimator::new(8),
        QuadraticUtility { optimum: 0.37 },
    )
    .with_grid_size(9)
    .with_refinement_rounds(6);

    let best = optimizer
        .optimize(&DummyModel, ContinuousDesignSpace::new(0.0, 1.0).unwrap())
        .unwrap();

    assert!((best.design - 0.37).abs() < 0.02);
}

#[test]
fn sequential_mapping_moves_the_next_probe_after_an_observation() {
    let optimizer =
        ContinuousDesignOptimizer::new(MonteCarloEstimator::new(128), ExpectedInformationGain)
            .with_grid_size(9)
            .with_refinement_rounds(5);

    let space = ContinuousDesignSpace::new(0.0, 1.0).unwrap();
    let mut session = SequentialDesignSession::new(SplitMappingModel::new(&[0.2, 0.4, 0.6, 0.8]));

    let first = session.choose_continuous(&optimizer, space).unwrap();
    assert!((first.design - 0.5).abs() < 0.1);

    session.update(&first.design, &true);
    let second = session.choose_continuous(&optimizer, space).unwrap();

    assert!(second.design > first.design + 0.1);
    assert!((second.design - 0.7).abs() < 0.12);
}

#[test]
fn session_records_history_and_updates_model() {
    let optimizer =
        ContinuousDesignOptimizer::new(MonteCarloEstimator::new(128), ExpectedInformationGain)
            .with_grid_size(9)
            .with_refinement_rounds(5);
    let space = ContinuousDesignSpace::new(0.0, 1.0).unwrap();
    let mut session = SequentialDesignSession::new(SplitMappingModel::new(&[0.2, 0.4, 0.6, 0.8]));

    let first = session.choose_continuous(&optimizer, space).unwrap();
    session.update(&first.design, &true);

    assert_eq!(session.step_count(), 1);
    assert_eq!(session.history()[0].step_index, 0);
    assert_eq!(session.history()[0].observation, true);
    let expected_support = [0.2, 0.4, 0.6, 0.8]
        .into_iter()
        .filter(|location| first.design <= *location)
        .collect::<Vec<_>>();
    assert_eq!(session.model().support, expected_support);
}
