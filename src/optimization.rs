use crate::{BayesianModel, BoedError, DesignEvaluation, MonteCarloEstimator, UtilityFunction};

/// The best design found within a finite candidate set.
pub type OptimizationResult<D> = DesignEvaluation<D>;

/// Optimizes expected utility over a finite set of candidate designs.
#[derive(Debug, Clone)]
pub struct DesignOptimizer<U> {
    estimator: MonteCarloEstimator,
    utility: U,
}

impl<U> DesignOptimizer<U> {
    pub fn new(estimator: MonteCarloEstimator, utility: U) -> Self {
        Self { estimator, utility }
    }

    pub fn estimator(&self) -> MonteCarloEstimator {
        self.estimator
    }

    pub fn utility(&self) -> &U {
        &self.utility
    }

    pub fn optimize<M>(
        &self,
        model: &M,
        candidates: &[M::Design],
    ) -> Result<OptimizationResult<M::Design>, BoedError>
    where
        M: BayesianModel,
        M::Design: Clone,
        U: UtilityFunction<M>,
    {
        let mut evaluations = candidates
            .iter()
            .cloned()
            .map(|design| self.estimator.evaluate(model, design, &self.utility));

        let mut best = match evaluations.next() {
            Some(first) => first?,
            None => return Err(BoedError::EmptyCandidateSet),
        };

        for evaluation in evaluations {
            let evaluation = evaluation?;
            if evaluation.expected_utility > best.expected_utility {
                best = evaluation;
            }
        }

        Ok(best)
    }
}
