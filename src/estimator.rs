use crate::{BayesianModel, BoedError, UtilityFunction};

/// The estimated utility of a design.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignEvaluation<D> {
    pub design: D,
    pub expected_utility: f64,
}

/// A simple Monte Carlo expected utility estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonteCarloEstimator {
    sample_count: usize,
}

impl MonteCarloEstimator {
    pub fn new(sample_count: usize) -> Self {
        Self { sample_count }
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    pub fn estimate<M, U>(
        &self,
        model: &M,
        design: &M::Design,
        utility: &U,
    ) -> Result<f64, BoedError>
    where
        M: BayesianModel,
        U: UtilityFunction<M>,
    {
        if self.sample_count == 0 {
            return Err(BoedError::ZeroSamples);
        }

        let sampled_parameters: Vec<M::Parameter> = (0..self.sample_count)
            .map(|draw_index| model.sample_prior(draw_index))
            .collect();

        let total_utility = sampled_parameters
            .iter()
            .enumerate()
            .map(|(draw_index, parameter)| {
                let observation = model.sample_observation(
                    design,
                    parameter,
                    observation_draw_index(draw_index, self.sample_count),
                );
                let evidence_parameters =
                    sample_evidence_parameters(model, draw_index, self.sample_count);
                utility.utility(model, design, parameter, &observation, &evidence_parameters)
            })
            .sum::<f64>();

        Ok(total_utility / self.sample_count as f64)
    }

    pub fn evaluate<M, U>(
        &self,
        model: &M,
        design: M::Design,
        utility: &U,
    ) -> Result<DesignEvaluation<M::Design>, BoedError>
    where
        M: BayesianModel,
        U: UtilityFunction<M>,
    {
        let expected_utility = self.estimate(model, &design, utility)?;
        Ok(DesignEvaluation {
            design,
            expected_utility,
        })
    }
}

fn observation_draw_index(draw_index: usize, sample_count: usize) -> usize {
    sample_count
        .saturating_mul(sample_count.saturating_add(1))
        .saturating_add(draw_index)
}

fn sample_evidence_parameters<M: BayesianModel>(
    model: &M,
    outer_draw_index: usize,
    sample_count: usize,
) -> Vec<M::Parameter> {
    let start = sample_count.saturating_mul(outer_draw_index.saturating_add(1));
    (0..sample_count)
        .map(|inner_offset| model.sample_prior(start.saturating_add(inner_offset)))
        .collect()
}
