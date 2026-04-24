use crate::model::BayesianModel;

/// A utility function for Bayesian optimal experimental design.
pub trait UtilityFunction<M: BayesianModel> {
    /// Computes the utility of a simulated parameter-observation pair under a
    /// candidate design.
    fn utility(
        &self,
        model: &M,
        design: &M::Design,
        parameter: &M::Parameter,
        observation: &M::Observation,
        evidence_parameters: &[M::Parameter],
    ) -> f64;
}

/// Expected information gain utility:
///
/// `log p(y | theta, d) - log p(y | d)`
#[derive(Debug, Clone, Copy, Default)]
pub struct ExpectedInformationGain;

impl<M> UtilityFunction<M> for ExpectedInformationGain
where
    M: BayesianModel,
{
    fn utility(
        &self,
        model: &M,
        design: &M::Design,
        parameter: &M::Parameter,
        observation: &M::Observation,
        evidence_parameters: &[M::Parameter],
    ) -> f64 {
        let log_joint = model.log_likelihood(design, parameter, observation);

        let log_evidence = log_mean_exp(
            evidence_parameters
                .iter()
                .map(|parameter| model.log_likelihood(design, parameter, observation)),
        );

        log_joint - log_evidence
    }
}

fn log_mean_exp(values: impl IntoIterator<Item = f64>) -> f64 {
    let collected: Vec<f64> = values.into_iter().collect();
    debug_assert!(!collected.is_empty());

    let max = collected.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if max == f64::NEG_INFINITY {
        return f64::NEG_INFINITY;
    }

    let sum_exp = collected.iter().map(|v| (v - max).exp()).sum::<f64>();

    max + (sum_exp / collected.len() as f64).ln()
}

#[cfg(test)]
mod tests {
    use super::log_mean_exp;

    #[test]
    fn log_mean_exp_matches_known_value() {
        let values = [0.0_f64.ln(), 1.0_f64.ln(), 4.0_f64.ln()];
        let result = log_mean_exp(values);
        let expected = (5.0_f64 / 3.0_f64).ln();
        assert!((result - expected).abs() < 1e-12);
    }

    #[test]
    fn log_mean_exp_handles_all_negative_infinity() {
        let values = [f64::NEG_INFINITY, f64::NEG_INFINITY];
        let result = log_mean_exp(values);
        assert_eq!(result, f64::NEG_INFINITY);
    }
}
