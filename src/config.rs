/// Serializable configuration and message types for user-facing BOED
/// integrations such as notebooks, web APIs, and queue-driven runtimes.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudyConfig {
    pub study_name: Option<String>,
    pub model_type: String,
    pub objective: ObjectiveSpec,
    pub design_space: DesignSpaceSpec,
    pub prior: PriorSpec,
    pub stopping_rule: Option<StoppingRule>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudySummary {
    pub study_id: String,
    pub study_name: Option<String>,
    pub model_type: String,
    pub objective: ObjectiveSpec,
    pub step_count: usize,
    pub status: StudyStatus,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudyStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObjectiveSpec {
    ExpectedInformationGain,
    PosteriorVarianceReduction {
        target: Option<Vec<String>>,
    },
    GoalOriented {
        metric: String,
        target: Option<ObservationValue>,
    },
    Custom {
        name: String,
        parameters: Metadata,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DesignSpaceSpec {
    FiniteSet {
        points: Vec<DesignPoint>,
    },
    ContinuousBox {
        bounds: Vec<[f64; 2]>,
    },
    Mixed {
        continuous: Vec<ContinuousDimension>,
        discrete: Vec<DiscreteDimension>,
    },
    Constrained {
        base: Box<DesignSpaceSpec>,
        constraints: Vec<ConstraintSpec>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignPoint {
    pub values: Vec<DesignValue>,
    pub label: Option<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousDimension {
    pub name: String,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscreteDimension {
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConstraintSpec {
    LinearInequality {
        coefficients: Vec<f64>,
        rhs: f64,
    },
    ForbiddenRegion {
        description: String,
    },
    Dependency {
        if_dimension: String,
        equals: String,
        then_allowed: Vec<String>,
    },
    Custom {
        name: String,
        parameters: Metadata,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PriorSpec {
    MultivariateNormal {
        mean: Vec<f64>,
        covariance: Vec<Vec<f64>>,
        parameter_names: Vec<String>,
    },
    Independent {
        marginals: Vec<DistributionSpec>,
    },
    DiscreteParticles {
        particles: Vec<ParameterVector>,
        weights: Vec<f64>,
    },
    DiscreteSupport {
        support: Vec<ObservationValue>,
    },
    Custom {
        name: String,
        parameters: Metadata,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DistributionSpec {
    Normal {
        name: String,
        mean: f64,
        std_dev: f64,
    },
    LogNormal {
        name: String,
        mean: f64,
        std_dev: f64,
    },
    Uniform {
        name: String,
        lower: f64,
        upper: f64,
    },
    Beta {
        name: String,
        alpha: f64,
        beta: f64,
    },
    Categorical {
        name: String,
        values: Vec<String>,
        probabilities: Vec<f64>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParameterVector {
    Continuous {
        values: Vec<f64>,
        names: Vec<String>,
    },
    Named {
        values: Vec<NamedValue>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedValue {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposalRecord {
    pub study_id: String,
    pub step_index: usize,
    pub design: Vec<DesignValue>,
    pub expected_utility: f64,
    pub objective: ObjectiveSpec,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationRecord {
    pub study_id: String,
    pub step_index: usize,
    pub design: Vec<DesignValue>,
    pub observation: ObservationValue,
    pub source: ObservationSource,
    pub recorded_at: Option<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PosteriorSummary {
    pub study_id: String,
    pub step_index: usize,
    pub representation: PosteriorRepresentation,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PosteriorRepresentation {
    Particles {
        particles: Vec<ParameterVector>,
        weights: Vec<f64>,
    },
    Moments {
        mean: Vec<f64>,
        covariance: Vec<Vec<f64>>,
        parameter_names: Vec<String>,
    },
    SummaryText {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationSource {
    ManualEntry,
    Notebook,
    Api,
    Queue,
    Simulator,
    Device { device_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DesignValue {
    Integer(i64),
    Number(f64),
    Text(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ObservationValue {
    Integer(i64),
    Scalar(f64),
    Boolean(bool),
    Text(String),
    Vector(Vec<f64>),
    Named(Metadata),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoppingRule {
    MaxSteps { steps: usize },
    UtilityThreshold { threshold: String },
    ConfidenceTarget { metric: String, threshold: String },
    Manual,
}

pub type Metadata = std::collections::BTreeMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_web_study_config_shape() {
        let config = StudyConfig {
            study_name: Some("cave-mapping".into()),
            model_type: "boundary_mapping".into(),
            objective: ObjectiveSpec::ExpectedInformationGain,
            design_space: DesignSpaceSpec::ContinuousBox {
                bounds: vec![[0.0, 1.0], [0.0, 1.0]],
            },
            prior: PriorSpec::DiscreteParticles {
                particles: vec![
                    ParameterVector::Continuous {
                        values: vec![0.2, 0.7],
                        names: vec!["x".into(), "y".into()],
                    },
                    ParameterVector::Continuous {
                        values: vec![0.8, 0.3],
                        names: vec!["x".into(), "y".into()],
                    },
                ],
                weights: vec![0.5, 0.5],
            },
            stopping_rule: Some(StoppingRule::MaxSteps { steps: 20 }),
            metadata: Metadata::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"model_type\":\"boundary_mapping\""));
        assert!(json.contains("\"expected_information_gain\""));
        assert!(json.contains("\"continuous_box\""));
    }

    #[test]
    fn round_trips_proposal_and_observation_records() {
        let proposal = ProposalRecord {
            study_id: "study-123".into(),
            step_index: 4,
            design: vec![
                DesignValue::Number(12.5),
                DesignValue::Number(0.8),
                DesignValue::Integer(42),
            ],
            expected_utility: 1.12,
            objective: ObjectiveSpec::ExpectedInformationGain,
            metadata: Metadata::new(),
        };

        let observation = ObservationRecord {
            study_id: "study-123".into(),
            step_index: 4,
            design: proposal.design.clone(),
            observation: ObservationValue::Named(
                [
                    ("kind".to_string(), "scalar".to_string()),
                    ("value".to_string(), "0.37".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
            source: ObservationSource::Queue,
            recorded_at: Some("2026-04-24T18:00:00Z".into()),
            metadata: Metadata::new(),
        };

        let proposal_json = serde_json::to_string(&proposal).unwrap();
        let observation_json = serde_json::to_string(&observation).unwrap();

        let proposal_back: ProposalRecord = serde_json::from_str(&proposal_json).unwrap();
        let observation_back: ObservationRecord = serde_json::from_str(&observation_json).unwrap();

        assert_eq!(proposal, proposal_back);
        assert_eq!(observation, observation_back);
    }
}
