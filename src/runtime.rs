use crate::{
    BayesianModel, BoedError, ContinuousDesignOptimizer, ContinuousDesignSpace, DesignOptimizer,
    DesignSpaceSpec, DesignValue, ExpectedInformationGain, Metadata, MonteCarloEstimator,
    ObjectiveSpec, ObservationRecord, ObservationSource, ObservationValue, PosteriorRepresentation,
    PosteriorSummary, PosteriorUpdate, PriorSpec, ProposalRecord, SequentialDesignSession,
    StudyConfig, StudyStatus, StudySummary,
};
use std::sync::atomic::{AtomicUsize, Ordering};

static STUDY_COUNTER: AtomicUsize = AtomicUsize::new(1);

pub struct StudySession {
    study_id: String,
    config: StudyConfig,
    status: StudyStatus,
    runtime: Box<dyn StudyRuntime>,
}

type RuntimeBuilder = fn(&StudyConfig) -> Result<Box<dyn StudyRuntime>, BoedError>;

const RUNTIME_BUILDERS: &[(&str, RuntimeBuilder)] =
    &[("boundary_mapping", build_boundary_mapping_runtime)];

trait StudyRuntime {
    fn propose_next(
        &self,
        study_id: &str,
        objective: &ObjectiveSpec,
    ) -> Result<ProposalRecord, BoedError>;
    fn observe(&mut self, observation: ObservationRecord) -> Result<(), BoedError>;
    fn posterior_summary(&self, study_id: &str) -> PosteriorSummary;
    fn step_count(&self) -> usize;
    fn history(&self, study_id: &str) -> Vec<ObservationRecord>;
}

#[derive(Debug, Clone)]
struct BoundaryMappingStudy {
    session: SequentialDesignSession<BoundaryMappingModel>,
    estimator: MonteCarloEstimator,
    continuous_space: Option<ContinuousDesignSpace>,
    finite_candidates: Option<Vec<f64>>,
}

#[derive(Debug, Clone)]
struct BoundaryMappingModel {
    support: Vec<f64>,
}

impl BoundaryMappingModel {
    fn new(support: Vec<f64>) -> Result<Self, BoedError> {
        if support.is_empty() {
            return Err(BoedError::InvalidStudyConfig(
                "boundary_mapping prior support must not be empty",
            ));
        }

        Ok(Self { support })
    }
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

impl StudySession {
    pub fn from_config(config: StudyConfig) -> Result<Self, BoedError> {
        let study_id = format!("study-{}", STUDY_COUNTER.fetch_add(1, Ordering::Relaxed));
        let runtime = build_runtime(&config)?;

        Ok(Self {
            study_id,
            config,
            status: StudyStatus::Active,
            runtime,
        })
    }

    pub fn study_id(&self) -> &str {
        &self.study_id
    }

    pub fn config(&self) -> &StudyConfig {
        &self.config
    }

    pub fn propose_next(&self) -> Result<ProposalRecord, BoedError> {
        self.runtime
            .propose_next(&self.study_id, &self.config.objective)
    }

    pub fn observe(&mut self, observation: ObservationRecord) -> Result<(), BoedError> {
        if observation.study_id != self.study_id {
            return Err(BoedError::InvalidObservation);
        }

        self.runtime.observe(observation)
    }

    pub fn posterior_summary(&self) -> PosteriorSummary {
        self.runtime.posterior_summary(&self.study_id)
    }

    pub fn summary(&self) -> StudySummary {
        StudySummary {
            study_id: self.study_id.clone(),
            study_name: self.config.study_name.clone(),
            model_type: self.config.model_type.clone(),
            objective: self.config.objective.clone(),
            step_count: self.runtime.step_count(),
            status: self.status.clone(),
            metadata: self.config.metadata.clone(),
        }
    }

    pub fn history(&self) -> Vec<ObservationRecord> {
        self.runtime.history(&self.study_id)
    }

    pub fn registered_model_types() -> &'static [&'static str] {
        &["boundary_mapping"]
    }
}

fn build_runtime(config: &StudyConfig) -> Result<Box<dyn StudyRuntime>, BoedError> {
    for (model_type, builder) in RUNTIME_BUILDERS {
        if config.model_type == *model_type {
            return builder(config);
        }
    }

    Err(BoedError::UnsupportedStudyConfig(
        "no registered runtime supports this model_type",
    ))
}

fn build_boundary_mapping_runtime(
    config: &StudyConfig,
) -> Result<Box<dyn StudyRuntime>, BoedError> {
    Ok(Box::new(BoundaryMappingStudy::from_config(config)?))
}

impl StudyRuntime for BoundaryMappingStudy {
    fn propose_next(
        &self,
        study_id: &str,
        objective: &ObjectiveSpec,
    ) -> Result<ProposalRecord, BoedError> {
        BoundaryMappingStudy::propose_next(self, study_id, objective)
    }

    fn observe(&mut self, observation: ObservationRecord) -> Result<(), BoedError> {
        BoundaryMappingStudy::observe(self, observation)
    }

    fn posterior_summary(&self, study_id: &str) -> PosteriorSummary {
        BoundaryMappingStudy::posterior_summary(self, study_id)
    }

    fn step_count(&self) -> usize {
        self.session.step_count()
    }

    fn history(&self, study_id: &str) -> Vec<ObservationRecord> {
        BoundaryMappingStudy::history(self, study_id)
    }
}

impl BoundaryMappingStudy {
    fn from_config(config: &StudyConfig) -> Result<Self, BoedError> {
        if config.objective != ObjectiveSpec::ExpectedInformationGain {
            return Err(BoedError::UnsupportedStudyConfig(
                "boundary_mapping currently supports only the expected_information_gain objective",
            ));
        }

        let support = match &config.prior {
            PriorSpec::DiscreteSupport { support } => support
                .iter()
                .map(parse_scalar_observation)
                .collect::<Result<Vec<_>, _>>()?,
            PriorSpec::DiscreteParticles { particles, .. } => particles
                .iter()
                .map(parse_1d_particle)
                .collect::<Result<Vec<_>, _>>()?,
            _ => {
                return Err(BoedError::UnsupportedStudyConfig(
                    "boundary_mapping supports only discrete_support or discrete_particles priors",
                ));
            }
        };

        let (continuous_space, finite_candidates) = match &config.design_space {
            DesignSpaceSpec::ContinuousBox { bounds } => {
                if bounds.len() != 1 {
                    return Err(BoedError::UnsupportedStudyConfig(
                        "boundary_mapping continuous design space must be one-dimensional",
                    ));
                }
                (
                    Some(ContinuousDesignSpace::new(bounds[0][0], bounds[0][1])?),
                    None,
                )
            }
            DesignSpaceSpec::FiniteSet { points } => (
                None,
                Some(
                    points
                        .iter()
                        .map(parse_1d_design_point)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
            _ => {
                return Err(BoedError::UnsupportedStudyConfig(
                    "boundary_mapping supports only one-dimensional continuous_box or finite_set designs",
                ));
            }
        };

        let estimator = MonteCarloEstimator::new(128);
        let model = BoundaryMappingModel::new(support)?;

        Ok(Self {
            session: SequentialDesignSession::new(model),
            estimator,
            continuous_space,
            finite_candidates,
        })
    }

    fn propose_next(
        &self,
        study_id: &str,
        objective: &ObjectiveSpec,
    ) -> Result<ProposalRecord, BoedError> {
        let metadata = Metadata::new();

        if let Some(space) = self.continuous_space {
            let result = self
                .session
                .choose_continuous(&self.continuous_optimizer(), space)?;
            return Ok(ProposalRecord {
                study_id: study_id.to_string(),
                step_index: self.session.step_count(),
                design: vec![DesignValue::Number(result.design)],
                expected_utility: result.expected_utility,
                objective: objective.clone(),
                metadata,
            });
        }

        if let Some(candidates) = &self.finite_candidates {
            let result = self
                .session
                .choose_from_candidates(&self.finite_optimizer(), candidates)?;
            return Ok(ProposalRecord {
                study_id: study_id.to_string(),
                step_index: self.session.step_count(),
                design: vec![DesignValue::Number(result.design)],
                expected_utility: result.expected_utility,
                objective: objective.clone(),
                metadata,
            });
        }

        Err(BoedError::InvalidStudyConfig(
            "study has no configured design space",
        ))
    }

    fn observe(&mut self, observation: ObservationRecord) -> Result<(), BoedError> {
        let design = parse_scalar_design(&observation.design)?;
        let observed = parse_boolean_observation(&observation.observation)?;
        self.session.update(&design, &observed);
        Ok(())
    }

    fn posterior_summary(&self, study_id: &str) -> PosteriorSummary {
        PosteriorSummary {
            study_id: study_id.to_string(),
            step_index: self.session.step_count(),
            representation: PosteriorRepresentation::Particles {
                particles: self
                    .session
                    .model()
                    .support
                    .iter()
                    .copied()
                    .map(|value| crate::ParameterVector::Continuous {
                        values: vec![value],
                        names: vec!["boundary".to_string()],
                    })
                    .collect(),
                weights: vec![
                    1.0 / self.session.model().support.len() as f64;
                    self.session.model().support.len()
                ],
            },
            metadata: Metadata::new(),
        }
    }

    fn history(&self, study_id: &str) -> Vec<ObservationRecord> {
        self.session
            .history()
            .iter()
            .map(|record| ObservationRecord {
                study_id: study_id.to_string(),
                step_index: record.step_index,
                design: vec![DesignValue::Number(record.design)],
                observation: ObservationValue::Boolean(record.observation),
                source: ObservationSource::Api,
                recorded_at: None,
                metadata: Metadata::new(),
            })
            .collect()
    }

    fn continuous_optimizer(&self) -> ContinuousDesignOptimizer<ExpectedInformationGain> {
        ContinuousDesignOptimizer::new(self.estimator, ExpectedInformationGain)
            .with_grid_size(9)
            .with_refinement_rounds(5)
    }

    fn finite_optimizer(&self) -> DesignOptimizer<ExpectedInformationGain> {
        DesignOptimizer::new(self.estimator, ExpectedInformationGain)
    }
}

fn parse_1d_design_point(point: &crate::DesignPoint) -> Result<f64, BoedError> {
    parse_scalar_design(&point.values)
}

fn parse_scalar_design(design: &[DesignValue]) -> Result<f64, BoedError> {
    if design.len() != 1 {
        return Err(BoedError::InvalidStudyConfig(
            "expected a one-dimensional design value",
        ));
    }

    match &design[0] {
        DesignValue::Integer(value) => Ok(*value as f64),
        DesignValue::Number(value) => Ok(*value),
        _ => Err(BoedError::InvalidStudyConfig(
            "expected a numeric design value",
        )),
    }
}

fn parse_1d_particle(parameter: &crate::ParameterVector) -> Result<f64, BoedError> {
    match parameter {
        crate::ParameterVector::Continuous { values, .. } if values.len() == 1 => Ok(values[0]),
        crate::ParameterVector::Named { values } if values.len() == 1 => Ok(values[0].value),
        _ => Err(BoedError::InvalidStudyConfig(
            "boundary_mapping particles must be one-dimensional",
        )),
    }
}

fn parse_scalar_observation(observation: &ObservationValue) -> Result<f64, BoedError> {
    match observation {
        ObservationValue::Integer(value) => Ok(*value as f64),
        ObservationValue::Scalar(value) => Ok(*value),
        _ => Err(BoedError::InvalidStudyConfig(
            "boundary_mapping discrete support must contain numeric values",
        )),
    }
}

fn parse_boolean_observation(observation: &ObservationValue) -> Result<bool, BoedError> {
    match observation {
        ObservationValue::Boolean(value) => Ok(*value),
        _ => Err(BoedError::InvalidObservation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DesignSpaceSpec, ObjectiveSpec, ObservationSource, ObservationValue, PriorSpec};

    fn boundary_mapping_config() -> StudyConfig {
        StudyConfig {
            study_name: Some("mapping".into()),
            model_type: "boundary_mapping".into(),
            objective: ObjectiveSpec::ExpectedInformationGain,
            design_space: DesignSpaceSpec::ContinuousBox {
                bounds: vec![[0.0, 1.0]],
            },
            prior: PriorSpec::DiscreteSupport {
                support: vec![
                    ObservationValue::Scalar(0.2),
                    ObservationValue::Scalar(0.4),
                    ObservationValue::Scalar(0.6),
                    ObservationValue::Scalar(0.8),
                ],
            },
            stopping_rule: None,
            metadata: Metadata::new(),
        }
    }

    #[test]
    fn constructs_session_from_supported_config() {
        let session = StudySession::from_config(boundary_mapping_config()).unwrap();
        let proposal = session.propose_next().unwrap();

        assert_eq!(proposal.step_index, 0);
        assert_eq!(proposal.objective, ObjectiveSpec::ExpectedInformationGain);
        assert_eq!(proposal.design.len(), 1);
    }

    #[test]
    fn updates_session_from_observation_record() {
        let mut session = StudySession::from_config(boundary_mapping_config()).unwrap();
        let proposal = session.propose_next().unwrap();

        session
            .observe(ObservationRecord {
                study_id: session.study_id().to_string(),
                step_index: proposal.step_index,
                design: proposal.design.clone(),
                observation: ObservationValue::Boolean(true),
                source: ObservationSource::ManualEntry,
                recorded_at: None,
                metadata: Metadata::new(),
            })
            .unwrap();

        assert_eq!(session.summary().step_count, 1);
        assert_eq!(session.history().len(), 1);
        match session.posterior_summary().representation {
            PosteriorRepresentation::Particles { particles, .. } => {
                assert!(particles.len() < 4);
            }
            _ => panic!("expected particle posterior"),
        }
    }

    #[test]
    fn exposes_registered_model_types() {
        assert_eq!(
            StudySession::registered_model_types(),
            &["boundary_mapping"]
        );
    }
}
