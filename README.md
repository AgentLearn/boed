# boed

`boed` is a Rust crate for Bayesian Optimal Experimental Design (BOED).

The crate currently provides:

- core abstractions for Bayesian models and candidate designs
- nested Monte Carlo estimation of expected utilities
- a built-in expected information gain utility
- a simple exhaustive optimizer over finite design sets
- a bounded continuous optimizer for one-dimensional interactive design loops

## Status

This is an early but usable foundation crate. The API is designed to make it easy to add:

- custom priors and simulators
- new utility functions
- smarter optimizers
- parallel or adaptive estimators

The expected information gain estimator uses one set of prior draws to
generate synthetic observations and a separate set of prior draws to estimate
the marginal evidence term `p(y | d)`.

## Quick Example

```rust
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
        // Deterministic stand-in samples for the example.
        const OFFSETS: [f64; 5] = [-1.5, -0.5, 0.0, 0.5, 1.5];
        self.prior_mean + self.prior_std * OFFSETS[draw_index % OFFSETS.len()]
    }

    fn sample_observation(
        &self,
        design: &Self::Design,
        parameter: &Self::Parameter,
        draw_index: usize,
    ) -> Self::Observation {
        // Use a stream that is independent from `sample_prior`.
        const NOISE: [f64; 5] = [-0.2, 0.1, 0.0, -0.1, 0.2];
        let epsilon = self.noise_std * NOISE[draw_index % NOISE.len()];
        design * parameter + epsilon
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

let model = ScalarGaussianModel {
    prior_mean: 0.0,
    prior_std: 1.0,
    noise_std: 0.25,
};

let estimator = MonteCarloEstimator::new(64);
let utility = ExpectedInformationGain::default();
let optimizer = DesignOptimizer::new(estimator, utility);

let candidates = [0.25, 0.5, 1.0, 2.0];
let best = optimizer.optimize(&model, &candidates).unwrap();

assert_eq!(best.design, 2.0);
```

## Roadmap

Potential next steps for the crate:

- importance sampling and nested Monte Carlo variants
- optional `rand` integration for stochastic sampling

## Where We Left Off

If you are coming back to the project later, start here:

- Current project status: [docs/current_status.md](/Users/zarkobizaca/code/New%20project/docs/current_status.md)
- User-facing interface design: [docs/interface_design.md](/Users/zarkobizaca/code/New%20project/docs/interface_design.md)

Right now the crate has:

- low-level BOED traits and estimators
- finite and 1D continuous optimizers
- sequential update helpers
- serializable config and message types
- a high-level `StudySession::from_config(...)` runtime entry point

The current high-level runtime is intentionally narrow and validated around the
`boundary_mapping` model type, but the runtime itself is now registry-based so
new domains can be added cleanly.

## Continuous Interactive Design

For continuous controls such as position, dose, or measurement time, use
`ContinuousDesignOptimizer` over a bounded interval and re-run it after each
posterior update. This gives you a simple interactive BOED loop without having
to pre-enumerate all candidate designs.

```rust
use boed::{
    BayesianModel, ContinuousDesignOptimizer, ContinuousDesignSpace,
    ExpectedInformationGain, MonteCarloEstimator,
};

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

let model = ScalarGaussianModel {
    prior_mean: 0.0,
    prior_std: 1.0,
    noise_std: 0.25,
};

let optimizer = ContinuousDesignOptimizer::new(
    MonteCarloEstimator::new(128),
    ExpectedInformationGain,
)
.with_grid_size(9)
.with_refinement_rounds(6);

let best = optimizer
    .optimize(&model, ContinuousDesignSpace::new(0.0, 2.0).unwrap())
    .unwrap();

assert!(best.design > 1.8);
```

## Sequential Posterior Updates

For repeated interactive design, implement `PosteriorUpdate` for your model and
let `SequentialDesignSession` manage the loop state.

```rust
use boed::{
    BayesianModel, ContinuousDesignOptimizer, ContinuousDesignSpace, ExpectedInformationGain,
    MonteCarloEstimator, PosteriorUpdate, SequentialDesignSession,
};

#[derive(Clone, Debug)]
struct BoundaryMappingModel {
    support: Vec<f64>,
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
            .collect();

        Self { support }
    }
}

let optimizer = ContinuousDesignOptimizer::new(
    MonteCarloEstimator::new(128),
    ExpectedInformationGain,
)
.with_grid_size(9)
.with_refinement_rounds(5);

let mut session = SequentialDesignSession::new(BoundaryMappingModel {
    support: vec![0.2, 0.4, 0.6, 0.8],
});

let space = ContinuousDesignSpace::new(0.0, 1.0).unwrap();
let first = session.choose_continuous(&optimizer, space).unwrap();
session.update(&first.design, &true);
let second = session.choose_continuous(&optimizer, space).unwrap();

assert!(second.design > first.design);
```
