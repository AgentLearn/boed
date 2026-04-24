use crate::{BayesianModel, BoedError, MonteCarloEstimator, UtilityFunction};

/// A bounded one-dimensional continuous design space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuousDesignSpace {
    lower: f64,
    upper: f64,
}

impl ContinuousDesignSpace {
    pub fn new(lower: f64, upper: f64) -> Result<Self, BoedError> {
        if !lower.is_finite() || !upper.is_finite() || lower >= upper {
            return Err(BoedError::InvalidContinuousBounds);
        }

        Ok(Self { lower, upper })
    }

    pub fn lower(&self) -> f64 {
        self.lower
    }

    pub fn upper(&self) -> f64 {
        self.upper
    }
}

/// The best continuous design found within a bounded interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuousOptimizationResult {
    pub design: f64,
    pub expected_utility: f64,
    pub final_interval: (f64, f64),
    pub evaluations: usize,
}

/// A coarse-to-fine optimizer for bounded one-dimensional design spaces.
///
/// This is a practical fit for interactive BOED loops where the design is a
/// single continuous control variable and the optimizer is re-run after each
/// posterior update.
#[derive(Debug, Clone)]
pub struct ContinuousDesignOptimizer<U> {
    estimator: MonteCarloEstimator,
    utility: U,
    grid_size: usize,
    refinement_rounds: usize,
}

impl<U> ContinuousDesignOptimizer<U> {
    pub fn new(estimator: MonteCarloEstimator, utility: U) -> Self {
        Self {
            estimator,
            utility,
            grid_size: 9,
            refinement_rounds: 6,
        }
    }

    pub fn estimator(&self) -> MonteCarloEstimator {
        self.estimator
    }

    pub fn utility(&self) -> &U {
        &self.utility
    }

    pub fn grid_size(&self) -> usize {
        self.grid_size
    }

    pub fn refinement_rounds(&self) -> usize {
        self.refinement_rounds
    }

    pub fn with_grid_size(mut self, grid_size: usize) -> Self {
        self.grid_size = grid_size;
        self
    }

    pub fn with_refinement_rounds(mut self, refinement_rounds: usize) -> Self {
        self.refinement_rounds = refinement_rounds;
        self
    }

    pub fn optimize<M>(
        &self,
        model: &M,
        space: ContinuousDesignSpace,
    ) -> Result<ContinuousOptimizationResult, BoedError>
    where
        M: BayesianModel<Design = f64>,
        U: UtilityFunction<M>,
    {
        if self.grid_size < 3 {
            return Err(BoedError::InvalidRefinementGrid);
        }

        if self.refinement_rounds == 0 {
            return Err(BoedError::InvalidRefinementRounds);
        }

        let mut lower = space.lower();
        let mut upper = space.upper();
        let mut evaluations = 0usize;
        let mut best_design = lower;
        let mut best_expected_utility = f64::NEG_INFINITY;

        for _ in 0..self.refinement_rounds {
            let step = (upper - lower) / (self.grid_size - 1) as f64;
            let mut round_best_index = 0usize;
            let mut round_best_design = lower;
            let mut round_best_expected_utility = f64::NEG_INFINITY;

            for grid_index in 0..self.grid_size {
                let design = if grid_index + 1 == self.grid_size {
                    upper
                } else {
                    lower + step * grid_index as f64
                };

                let expected_utility = self.estimator.estimate(model, &design, &self.utility)?;
                evaluations += 1;

                if expected_utility > round_best_expected_utility {
                    round_best_index = grid_index;
                    round_best_design = design;
                    round_best_expected_utility = expected_utility;
                }
            }

            best_design = round_best_design;
            best_expected_utility = round_best_expected_utility;

            if step == 0.0 {
                break;
            }

            let next_lower = if round_best_index == 0 {
                lower
            } else {
                lower + step * (round_best_index - 1) as f64
            };
            let next_upper = if round_best_index + 1 >= self.grid_size {
                upper
            } else {
                lower + step * (round_best_index + 1) as f64
            };

            lower = next_lower;
            upper = next_upper;
        }

        Ok(ContinuousOptimizationResult {
            design: best_design,
            expected_utility: best_expected_utility,
            final_interval: (lower, upper),
            evaluations,
        })
    }
}
