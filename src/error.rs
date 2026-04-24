use core::fmt;

/// Errors returned by BOED estimation and optimization routines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoedError {
    EmptyCandidateSet,
    InvalidContinuousBounds,
    InvalidObservation,
    InvalidRefinementGrid,
    InvalidRefinementRounds,
    InvalidStudyConfig(&'static str),
    UnsupportedStudyConfig(&'static str),
    ZeroSamples,
}

impl fmt::Display for BoedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCandidateSet => f.write_str("candidate design set is empty"),
            Self::InvalidContinuousBounds => {
                f.write_str("continuous design bounds must be finite and ordered")
            }
            Self::InvalidObservation => f.write_str("observation does not match the study model"),
            Self::InvalidRefinementGrid => {
                f.write_str("continuous optimizer requires a refinement grid of at least 3 points")
            }
            Self::InvalidRefinementRounds => {
                f.write_str("continuous optimizer requires at least one refinement round")
            }
            Self::InvalidStudyConfig(message) => f.write_str(message),
            Self::UnsupportedStudyConfig(message) => f.write_str(message),
            Self::ZeroSamples => f.write_str("monte carlo estimator requires at least one sample"),
        }
    }
}

impl std::error::Error for BoedError {}
