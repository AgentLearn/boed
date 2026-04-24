/// A Bayesian model capable of prior sampling, data simulation, and likelihood
/// evaluation for candidate designs.
pub trait BayesianModel {
    type Design;
    type Parameter;
    type Observation;

    /// Produces the `draw_index`th sample from the prior.
    ///
    /// The trait intentionally leaves the sampling strategy open. Callers may
    /// implement deterministic quadrature-like sampling or stochastic draws.
    fn sample_prior(&self, draw_index: usize) -> Self::Parameter;

    /// Simulates an observation given a design, latent parameter, and
    /// observation draw index.
    ///
    /// Implementations should treat `draw_index` as belonging to a randomness
    /// stream that is independent from the one used by [`Self::sample_prior`].
    fn sample_observation(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        draw_index: usize,
    ) -> Self::Observation;

    /// Evaluates the log likelihood `log p(y | theta, d)`.
    fn log_likelihood(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        observation: &Self::Observation,
    ) -> f64;
}
