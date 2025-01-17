//! Mutators that can handle recursive types.
//!
//! There are two main mutators:
//! 1. [`RecursiveMutator`] is the top-level mutator for the recursive type
//! 2. [`RecurToMutator`] is the mutator used at points of recursion. It is essentially a weak reference to [`RecursiveMutator`]
//!
//! In practice, you will want to use the [`make_mutator!`](crate::make_mutator) procedural macro to create recursive mutators.
//! For example:
//! ```
//! # #![feature(no_coverage)]
//! use fuzzcheck::mutators::{option::OptionMutator, boxed::BoxMutator};
//! use fuzzcheck::mutators::recursive::{RecursiveMutator, RecurToMutator};
//! use fuzzcheck::DefaultMutator;
//! use fuzzcheck::make_mutator;
//!
//! #[derive(Clone)]
//! struct S {
//!     content: bool,
//!     next: Option<Box<S>> // the type recurses here
//! }
//!
//! make_mutator! {
//!     name: SMutator,
//!     recursive: true, // this is important
//!     default: false,
//!     type: struct S {
//!         content: bool,
//!         // We need to specify a concrete sub-mutator for this field to avoid creating an infinite type.
//!         // We use the standard Option and Box mutators, but replace what would be SMutator<M0, M1> by
//!         // RecurToMutator<SMutator<M0>>, which indicates that this is a point of recursion
//!         // and the mutator should be a weak reference to a RecursiveMutator
//!         // The M0 part refers to the mutator for the `content: bool` field.
//!         #[field_mutator(OptionMutator<Box<S>, BoxMutator<RecurToMutator<SMutator<M0>>>>)]
//!         next: Option<Box<S>>
//!     }
//! }
//! # fn main() {
//!
//! let s_mutator = RecursiveMutator::new(|mutator| {
//!     SMutator::new(
//!         /*content_mutator:*/ bool::default_mutator(),
//!         /*next_mutator:*/ OptionMutator::new(BoxMutator::new(RecurToMutator::from(mutator)))
//!     )
//! });
//! // s_mutator impl Mutator<S>
//! # }
//! ```

use crate::Mutator;
use std::{
    any::Any,
    fmt::Debug,
    rc::{Rc, Weak},
};

/// The ArbitraryStep that is used for recursive mutators
#[derive(Clone, Debug, PartialEq)]
pub enum RecursingArbitraryStep<AS> {
    Default,
    Initialized(AS),
}
impl<AS> Default for RecursingArbitraryStep<AS> {
    #[no_coverage]
    fn default() -> Self {
        Self::Default
    }
}

/**
A wrapper that allows a mutator to call itself recursively.

For example, it is used to provide mutators for types such as:
```
struct S {
    content: bool,
    // to mutate this field, a mutator must be able to recursively call itself
    next: Option<Box<S>>
}
```
`RecursiveMutator` is only the top-level type. It must be used in conjuction
with [`RecurToMutator`](crate::mutators::recursive::RecurToMutator) at points of recursion.
For example:
```
# #![feature(no_coverage)]
use fuzzcheck::DefaultMutator;
use fuzzcheck::mutators::{option::OptionMutator, boxed::BoxMutator};
use fuzzcheck::mutators::recursive::{RecursiveMutator, RecurToMutator};

# use fuzzcheck::make_mutator;
# #[derive(Clone)]
# struct S {
#     content: bool,
#     next: Option<Box<S>>
# }
# make_mutator! {
#     name: SMutator,
#     recursive: true,
#     default: false,
#     type: struct S {
#         content: bool,
#         #[field_mutator(OptionMutator<Box<S>, BoxMutator<RecurToMutator<SMutator<M0>>>>)]
#         next: Option<Box<S>>
#     }
# }
let s_mutator = RecursiveMutator::new(|mutator| {
    SMutator::new(
        /*content_mutator:*/ bool::default_mutator(),
        /*next_mutator:*/ OptionMutator::new(BoxMutator::new(RecurToMutator::from(mutator)))
    )
});
```
*/
pub struct RecursiveMutator<M> {
    pub mutator: Rc<M>,
    rng: fastrand::Rng,
}
impl<M> RecursiveMutator<M> {
    /// Create a new `RecursiveMutator` using a weak reference to itself.
    #[no_coverage]
    pub fn new(data_fn: impl FnOnce(&Weak<M>) -> M) -> Self {
        Self {
            mutator: Rc::new_cyclic(data_fn),
            rng: fastrand::Rng::new(),
        }
    }
}

/// A mutator that defers to a weak reference of a
/// [`RecursiveMutator`](crate::mutators::recursive::RecursiveMutator)
pub struct RecurToMutator<M> {
    reference: Weak<M>,
}
impl<M> From<&Weak<M>> for RecurToMutator<M> {
    #[no_coverage]
    fn from(reference: &Weak<M>) -> Self {
        Self {
            reference: reference.clone(),
        }
    }
}

impl<T, M> Mutator<T> for RecurToMutator<M>
where
    M: Mutator<T>,
    T: Clone + 'static,
{
    #[doc(hidden)]
    type Cache = <M as Mutator<T>>::Cache;
    #[doc(hidden)]
    type MutationStep = <M as Mutator<T>>::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = RecursingArbitraryStep<<M as Mutator<T>>::ArbitraryStep>;
    #[doc(hidden)]
    type UnmutateToken = <M as Mutator<T>>::UnmutateToken;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        RecursingArbitraryStep::Default
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.reference.upgrade().unwrap().validate_value(value)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.reference.upgrade().unwrap().default_mutation_step(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        std::f64::INFINITY
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        // should be the min complexity of the mutator
        if let Some(m) = self.reference.upgrade() {
            m.as_ref().min_complexity()
        } else {
            1.0 // not right, but easy hack for now
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.reference.upgrade().unwrap().complexity(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        match step {
            RecursingArbitraryStep::Default => {
                let mutator = self.reference.upgrade().unwrap();
                let inner_step = mutator.default_arbitrary_step();
                *step = RecursingArbitraryStep::Initialized(inner_step);
                self.ordered_arbitrary(step, max_cplx)
            }
            RecursingArbitraryStep::Initialized(inner_step) => self
                .reference
                .upgrade()
                .unwrap()
                .ordered_arbitrary(inner_step, max_cplx),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.reference.upgrade().unwrap().random_arbitrary(max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.reference
            .upgrade()
            .unwrap()
            .ordered_mutate(value, cache, step, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.reference.upgrade().unwrap().random_mutate(value, cache, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.reference.upgrade().unwrap().unmutate(value, cache, t)
    }

    #[doc(hidden)]
    type RecursingPartIndex = bool;
    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, _value: &T, _cache: &Self::Cache) -> Self::RecursingPartIndex {
        false
    }
    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(&self, parent: &N, value: &'a T, index: &mut Self::RecursingPartIndex) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        if *index {
            None
        } else {
            *index = true;
            let parent_any: &dyn Any = parent;
            if let Some(parent) = parent_any.downcast_ref::<RecursiveMutator<M>>() {
                if Rc::downgrade(&parent.mutator).ptr_eq(&self.reference) {
                    let v: &dyn Any = value;
                    let v = v.downcast_ref::<V>().unwrap();
                    Some(v)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecursiveMutatorMutationStep<MS, RPI> {
    recursing_part_index: Option<RPI>,
    mutation_step: MS,
}

pub enum RecursiveMutatorUnmutateToken<T, UnmutateToken> {
    Replace(T),
    Token(UnmutateToken),
}

impl<M, T: Clone + 'static> Mutator<T> for RecursiveMutator<M>
where
    M: Mutator<T>,
{
    type Cache = M::Cache;
    type MutationStep = RecursiveMutatorMutationStep<M::MutationStep, M::RecursingPartIndex>;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = RecursiveMutatorUnmutateToken<T, M::UnmutateToken>;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        let mutation_step = self.mutator.default_mutation_step(value, cache);
        let recursing_part_index = Some(self.default_recursing_part_index(value, cache));

        RecursiveMutatorMutationStep {
            mutation_step,
            recursing_part_index,
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        self.mutator.ordered_arbitrary(step, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.mutator.random_arbitrary(max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if let Some(recursing_part_index) = &mut step.recursing_part_index {
            if let Some(new) = self
                .mutator
                .recursing_part::<T, Self>(self, value, recursing_part_index)
            {
                let mut new = new.clone();
                let cache = self.validate_value(&new).unwrap();
                let cplx = self.complexity(&new, &cache);
                std::mem::swap(value, &mut new);
                let token = RecursiveMutatorUnmutateToken::Replace(new);
                Some((token, cplx))
            } else {
                step.recursing_part_index = None;
                self.ordered_mutate(value, cache, step, max_cplx)
            }
        } else {
            if let Some((token, cplx)) = self
                .mutator
                .ordered_mutate(value, cache, &mut step.mutation_step, max_cplx)
            {
                Some((RecursiveMutatorUnmutateToken::Token(token), cplx))
            } else {
                None
            }
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        if self.rng.usize(..100) == 0 {
            let mut recursing_part_index = self.default_recursing_part_index(value, cache);
            if let Some(new) = self
                .mutator
                .recursing_part::<T, Self>(self, value, &mut recursing_part_index)
            {
                let mut new = new.clone();
                let cache = self.validate_value(&new).unwrap();
                let cplx = self.complexity(&new, &cache);
                std::mem::swap(value, &mut new);
                let token = RecursiveMutatorUnmutateToken::Replace(new);
                return (token, cplx);
            }
        }
        let (token, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
        let token = RecursiveMutatorUnmutateToken::Token(token);
        (token, cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            RecursiveMutatorUnmutateToken::Replace(x) => {
                let _ = std::mem::replace(value, x);
            }
            RecursiveMutatorUnmutateToken::Token(t) => self.mutator.unmutate(value, cache, t),
        }
    }

    #[doc(hidden)]
    type RecursingPartIndex = M::RecursingPartIndex;

    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, value: &T, cache: &Self::Cache) -> Self::RecursingPartIndex {
        self.mutator.default_recursing_part_index(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(&self, parent: &N, value: &'a T, index: &mut Self::RecursingPartIndex) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        self.mutator.recursing_part::<V, N>(parent, value, index)
    }
}
