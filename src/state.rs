//! Domain trait contracts implemented by the host logic engine.

use std::error::Error;
use std::fmt::Debug;
use std::hash::Hash;

/// A variable or relation that the dilemma rule can branch on.
///
/// Blanket-implemented for every type that meets the bounds, so integer
/// indices, interned symbols, or small `Copy` structs all qualify.
pub trait LogicVariable: Copy + Eq + Hash + Send + Sync + Debug {}

impl<T> LogicVariable for T where T: Copy + Eq + Hash + Send + Sync + Debug {}

/// A value that can be assumed for a [`LogicVariable`].
///
/// Blanket-implemented for every type that meets the bounds, such as
/// `bool` or an equivalence-class handle.
pub trait LogicValue: Clone + Eq + Send + Sync + Debug {}

impl<T> LogicValue for T where T: Clone + Eq + Send + Sync + Debug {}

/// Whether a 0-saturation pass derived anything new.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SaturationStatus {
    /// The pass derived nothing; the state is at a fixpoint.
    Converged,
    /// The pass derived new facts; another pass may derive more.
    Changed,
}

/// A logical state that the dilemma engine can branch on and merge.
///
/// Implementations own their storage model (triplet tables, graphs,
/// bitsets). `Clone` is called once per branch, so states backed by
/// persistent, structurally shared data structures branch cheaply.
///
/// The engine's fixpoint loops assume that facts only accumulate: every
/// `assume` and `saturate_zero` makes the state stronger or leaves it
/// unchanged, and `intersect` never adds a fact missing from either side.
pub trait LogicState: Clone + Send + Sync {
    /// The variables this state can branch on.
    type Var: LogicVariable;
    /// The values a variable can be assumed to take.
    type Val: LogicValue;
    /// The contradiction reported when the state becomes inconsistent.
    type Error: Error + Send + Sync + 'static;

    /// Runs one pass of the local 0-saturation rules, such as structural
    /// equivalence propagation.
    ///
    /// # Errors
    ///
    /// Returns an error if the pass derives a contradiction.
    fn saturate_zero(&mut self) -> Result<SaturationStatus, Self::Error>;

    /// Injects a temporary assumption such as `v == true` or `a == b`.
    ///
    /// # Errors
    ///
    /// Returns an error if the assumption contradicts the state.
    fn assume(
        &mut self,
        variable: Self::Var,
        value: Self::Val,
    ) -> Result<(), Self::Error>;

    /// Returns the facts true in both `self` and `other`.
    fn intersect(&self, other: &Self) -> Self;

    /// Records a derivation step for proof-certificate generation.
    ///
    /// The default implementation ignores the step.
    fn record_proof_step(&mut self, _step_details: &str) {}
}
