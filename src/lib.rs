//! `boed` provides a lightweight foundation for Bayesian Optimal Experimental
//! Design in Rust.
//!
//! The crate is centered around five concepts:
//!
//! - [`BayesianModel`]: prior sampling, simulation, and likelihood evaluation
//! - [`UtilityFunction`]: objective to maximize for a candidate design
//! - [`MonteCarloEstimator`]: expected utility estimator
//! - [`DesignOptimizer`]: search over a finite set of candidate designs
//! - [`ContinuousDesignOptimizer`]: coarse-to-fine search over a bounded
//!   one-dimensional design space
//! - [`SequentialDesignSession`]: a helper for repeated plan-observe-update
//!   BOED loops

mod config;
mod continuous;
mod error;
mod estimator;
mod model;
mod optimization;
mod runtime;
mod sequential;
mod utility;

pub use config::{
    ConstraintSpec, ContinuousDimension, DesignPoint, DesignSpaceSpec, DesignValue,
    DiscreteDimension, DistributionSpec, Metadata, NamedValue, ObjectiveSpec, ObservationRecord,
    ObservationSource, ObservationValue, ParameterVector, PosteriorRepresentation,
    PosteriorSummary, PriorSpec, ProposalRecord, StoppingRule, StudyConfig, StudyStatus,
    StudySummary,
};
pub use continuous::{
    ContinuousDesignOptimizer, ContinuousDesignSpace, ContinuousOptimizationResult,
};
pub use error::BoedError;
pub use estimator::{DesignEvaluation, MonteCarloEstimator};
pub use model::BayesianModel;
pub use optimization::{DesignOptimizer, OptimizationResult};
pub use runtime::StudySession;
pub use sequential::{PosteriorUpdate, SequentialDesignRecord, SequentialDesignSession};
pub use utility::{ExpectedInformationGain, UtilityFunction};
