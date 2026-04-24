use crate::{
    BayesianModel, BoedError, ContinuousDesignOptimizer, ContinuousDesignSpace,
    ContinuousOptimizationResult, DesignOptimizer, OptimizationResult, UtilityFunction,
};

/// Updates a BOED model after observing data from a chosen design.
///
/// This trait lets the library own the "plan, observe, update" loop while the
/// user remains responsible for specifying how an observation changes the
/// posterior represented by their model type.
pub trait PosteriorUpdate: BayesianModel + Sized {
    fn posterior_update(&self, design: &Self::Design, observation: &Self::Observation) -> Self;
}

/// A single completed BOED step in a sequential design loop.
#[derive(Debug, Clone, PartialEq)]
pub struct SequentialDesignRecord<D, O> {
    pub step_index: usize,
    pub design: D,
    pub observation: O,
}

/// Maintains the current posterior model and step history for sequential BOED.
#[derive(Debug, Clone)]
pub struct SequentialDesignSession<M: BayesianModel> {
    model: M,
    history: Vec<SequentialDesignRecord<M::Design, M::Observation>>,
}

impl<M> SequentialDesignSession<M>
where
    M: PosteriorUpdate,
{
    pub fn new(model: M) -> Self {
        Self {
            model,
            history: Vec::new(),
        }
    }

    pub fn model(&self) -> &M {
        &self.model
    }

    pub fn into_model(self) -> M {
        self.model
    }

    pub fn history(&self) -> &[SequentialDesignRecord<M::Design, M::Observation>] {
        &self.history
    }

    pub fn step_count(&self) -> usize {
        self.history.len()
    }

    pub fn choose_from_candidates<U>(
        &self,
        optimizer: &DesignOptimizer<U>,
        candidates: &[M::Design],
    ) -> Result<OptimizationResult<M::Design>, BoedError>
    where
        M::Design: Clone,
        U: UtilityFunction<M>,
    {
        optimizer.optimize(&self.model, candidates)
    }

    pub fn choose_continuous<U>(
        &self,
        optimizer: &ContinuousDesignOptimizer<U>,
        space: ContinuousDesignSpace,
    ) -> Result<ContinuousOptimizationResult, BoedError>
    where
        M: BayesianModel<Design = f64>,
        U: UtilityFunction<M>,
    {
        optimizer.optimize(&self.model, space)
    }

    pub fn update(&mut self, design: &M::Design, observation: &M::Observation)
    where
        M::Design: Clone,
        M::Observation: Clone,
    {
        let next_model = self.model.posterior_update(design, observation);
        let step_index = self.history.len();
        self.history.push(SequentialDesignRecord {
            step_index,
            design: design.clone(),
            observation: observation.clone(),
        });
        self.model = next_model;
    }
}
