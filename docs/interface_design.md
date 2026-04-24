# User-Facing Interface Design

This document describes how users could interact with `boed` in practice,
without locking the crate into a single application style. The goal is to make
the core library flexible enough for:

- Python notebooks used by researchers
- web backends that coordinate iterative experiments
- queue or event-driven systems for physical experiments
- robotics and mission-planning loops with asynchronous observations

## Product Shape

The crate should be treated as a layered system:

1. `boed-core`
   Rust traits and algorithms for models, priors, utilities, and optimization.

2. `boed-runtime`
   Session orchestration, persistence helpers, and event-oriented interfaces for
   repeated plan-observe-update workflows.

3. Integrations
   Thin adapters for:
   - Python notebooks via `pyo3`
   - HTTP services via Axum/Actix
   - queue consumers/producers via Kafka, NATS, SQS, or Redis streams

The current crate already covers most of `boed-core` and part of
`boed-runtime`.

## Primary User Workflows

### 1. Notebook / Research Workflow

Best for:

- drug development exploration
- athlete training strategy analysis
- mission simulation studies

User interaction:

1. define the model and prior
2. define allowed experiment designs
3. ask for the next recommended design
4. record a real or simulated result
5. update the posterior
6. repeat

Python-style surface:

```python
session = boed.Session.from_config(
    model=model,
    prior=prior,
    design_space=design_space,
    objective="expected_information_gain",
)

proposal = session.propose_next()
print(proposal.design, proposal.expected_utility)

session.observe(
    design=proposal.design,
    observation=lab_result,
)
```

### 2. Web Application Workflow

Best for:

- product teams managing experiment campaigns
- operators monitoring field systems
- collaboration across analysts and domain experts

User interaction:

1. create a study
2. upload initial configuration and priors
3. request the next design recommendation
4. enter or upload observed results
5. inspect history and current posterior summary

Recommended API shape:

- `POST /studies`
- `POST /studies/{id}/proposal`
- `POST /studies/{id}/observations`
- `GET /studies/{id}`
- `GET /studies/{id}/history`
- `GET /studies/{id}/posterior`

Example request:

```json
{
  "model_type": "boundary_mapping",
  "objective": "expected_information_gain",
  "design_space": {
    "kind": "continuous_box",
    "bounds": [[0.0, 1.0]]
  },
  "prior": {
    "kind": "discrete_support",
    "support": [0.2, 0.4, 0.6, 0.8]
  }
}
```

Example proposal response:

```json
{
  "step_index": 0,
  "design": [0.4],
  "expected_utility": 0.693,
  "metadata": {
    "optimizer": "continuous_grid_refinement",
    "evaluations": 45
  }
}
```

### 3. Queue / Event-Driven Workflow

Best for:

- automated lab systems
- drone fleets
- industrial calibration
- long-running mission planning services

User interaction:

1. a planner service emits a `design.proposed` event
2. an execution system performs the experiment
3. an observation arrives as `experiment.observed`
4. the session updates and emits another recommendation

Recommended event envelope:

```json
{
  "study_id": "study-123",
  "step_index": 4,
  "timestamp": "2026-04-24T18:00:00Z",
  "payload": {}
}
```

Proposal payload:

```json
{
  "design": [12.5, 0.8, 42.0],
  "expected_utility": 1.12,
  "objective": "expected_information_gain"
}
```

Observation payload:

```json
{
  "design": [12.5, 0.8, 42.0],
  "observation": {
    "kind": "scalar",
    "value": 0.37
  }
}
```

This is the right fit when observations are produced manually, by humans in a
dashboard, or by asynchronous systems that report back later.

## Core Concepts Users Need

From a user perspective, they should not need to think in terms of traits
first. They need five pieces of information:

1. State model
   What unknown parameters are we learning?

2. Prior
   What do we believe initially?

3. Design space
   What choices can we make next?

4. Observation model
   If we run a design, what data comes back?

5. Objective
   What counts as a good next experiment?

These should be configurable through a high-level config object.

Suggested Rust shape:

```rust
pub struct StudyConfig<M, P, D, O> {
    pub model: M,
    pub prior: P,
    pub design_space: D,
    pub objective: O,
    pub stopping_rule: Option<StoppingRule>,
}
```

## Proposed High-Level Rust Interface

The current low-level API is still valuable, but most users will want a more
direct entry point.

Suggested surface:

```rust
pub struct StudySession<M, S, U> {
    pub state: S,
    pub model: M,
    pub utility: U,
}

impl<M, S, U> StudySession<M, S, U> {
    pub fn propose_next(&self) -> Proposal;
    pub fn observe(&mut self, observation: ObservationRecord) -> UpdateSummary;
    pub fn posterior_summary(&self) -> PosteriorSummary;
    pub fn history(&self) -> &[StepRecord];
}
```

This higher-level API should internally use the lower-level BOED traits, but
present a simpler object model to end users.

## Design Space Interface

To support small modifications across many domains, the design space should be
explicitly modeled rather than buried in model code.

Suggested variants:

```rust
pub enum DesignSpace {
    FiniteSet(Vec<DesignPoint>),
    ContinuousBox {
        lower: Vec<f64>,
        upper: Vec<f64>,
    },
    Mixed {
        continuous: Vec<(f64, f64)>,
        discrete: Vec<Vec<String>>,
    },
    Constrained(Box<DesignSpace>, ConstraintSet),
}
```

This covers:

- drug development: dose, schedule, assay timing
- athlete training: intensity, duration, recovery spacing
- cave mapping: position, altitude, sensor mode
- mission planning: burn timing, angle, fuel allocation, instrument schedule

## Priors and Parameter Interfaces

Users will often have multidimensional parameters. Those should be first-class.

Suggested parameter representation:

```rust
pub enum ParameterVector {
    Continuous(Vec<f64>),
    Named(Vec<(String, f64)>),
}
```

Suggested prior variants:

```rust
pub enum PriorSpec {
    MultivariateNormal {
        mean: Vec<f64>,
        covariance: Vec<Vec<f64>>,
    },
    Independent {
        marginals: Vec<DistributionSpec>,
    },
    DiscreteParticles {
        particles: Vec<Vec<f64>>,
        weights: Vec<f64>,
    },
}
```

Particle priors are especially useful because they map well to BOED,
sequential inference, and robotics-style planning.

## Observation Ingestion Interface

The user should be able to provide results manually or asynchronously.

Suggested observation record:

```rust
pub struct ObservationRecord<D, O> {
    pub step_index: usize,
    pub design: D,
    pub observation: O,
    pub source: ObservationSource,
    pub recorded_at: String,
}

pub enum ObservationSource {
    ManualEntry,
    Notebook,
    Api,
    Queue,
    Simulator,
    Device(String),
}
```

This supports:

- a scientist typing assay output manually
- a coach uploading workout results
- a drone posting sensor packets later
- a mission simulator publishing delayed telemetry

## Domain Mapping

The same interface should work across the following domains with only model and
observation changes:

### Drug Development

- parameters: efficacy, toxicity, PK/PD coefficients
- design: dose, cadence, inclusion criteria, sampling times
- observation: biomarker readouts, adverse events, response rates

### Athlete Performance Training

- parameters: fatigue dynamics, adaptation rate, recovery sensitivity
- design: workout intensity, duration, taper schedule
- observation: split times, HRV, lactate, subjective readiness

### Cave Mapping with Drones

- parameters: occupancy map, hazard map, localization uncertainty
- design: waypoint, path segment, sensor settings
- observation: depth scans, images, gas readings, SLAM updates

### Mission Planning

- parameters: trajectory uncertainty, subsystem reliability, environmental risk
- design: maneuver timing, observation windows, instrument allocation
- observation: tracking data, fuel state, sensor measurements

## Recommended Near-Term Implementation Path

Instead of building every interface at once, the next practical layers should
be:

1. Stable high-level session config structs
2. Serializable proposal and observation records
3. A Python binding for notebook-first use
4. An HTTP example service
5. A queue-driven example for asynchronous updates

## What the End User Should Feel

The crate should feel like:

- "I describe what I know"
- "the system tells me the best next action"
- "I feed back what happened"
- "the posterior updates"
- "I repeat until my stopping rule is met"

That interaction model should stay the same whether the user is:

- in a Jupyter notebook
- in a browser dashboard
- in an automated lab
- operating a drone or mission-planning service
