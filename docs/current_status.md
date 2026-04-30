# Current Status

## What Exists

- BOED core traits and estimators are implemented.
- Expected information gain uses a nested Monte Carlo structure.
- Finite and 1D continuous optimizers are available.
- Sequential BOED loops are supported through `SequentialDesignSession`.
- Serializable config, proposal, observation, and posterior types exist.
- `StudySession::from_config(...)` now creates a runnable high-level session.

## Current High-Level Runtime Scope

The high-level runtime currently supports one concrete registered model:

- `boundary_mapping`

Supported config combinations for that runtime:

- objective: `expected_information_gain`
- priors: `discrete_support`, `discrete_particles` (1D)
- design spaces: `continuous_box` (1D), `finite_set` (1D numeric points)
- observations: boolean outcomes

This is intentionally narrow, but it proves the config-to-proposal-to-update
loop end to end.

## Important Files

- Core interfaces: [src/lib.rs](/Users/zarkobizaca/code/optimization/boed/src/lib.rs)
- Config/message types: [src/config.rs](/Users/zarkobizaca/code/optimization/boed/src/config.rs)
- High-level runtime session: [src/runtime.rs](/Users/zarkobizaca/code/optimization/boed/src/runtime.rs)
- Sequential session helper: [src/sequential.rs](/Users/zarkobizaca/code/optimization/boed/src/sequential.rs)
- Continuous optimizer: [src/continuous.rs](/Users/zarkobizaca/code/optimization/boed/src/continuous.rs)
- Interface design notes: [docs/interface_design.md](/Users/zarkobizaca/code/optimization/boed/docs/interface_design.md)
- Validation coverage: [tests/validation_suite.rs](/Users/zarkobizaca/code/optimization/boed/tests/validation_suite.rs)
- Example loop: [examples/interactive_mapping.rs](/Users/zarkobizaca/code/optimization/boed/examples/interactive_mapping.rs)

## Where We Left Off

The runtime is now registry-based rather than a single hard-coded constructor
branch. New domains should be added by registering another runtime builder,
instead of expanding one large `match` inside `StudySession::from_config(...)`.

The next likely extensions are:

1. Add another registered runtime for a richer domain, such as dose selection or training planning.
2. Move optimizer and sampler knobs into `StudyConfig`.
3. Add `StudySession::from_config_with_registry(...)` or a public registration surface for external crates.
4. Add a notebook or HTTP adapter on top of the config/runtime API.
