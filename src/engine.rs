//! Dilemma-rule drivers: single 1-saturation steps and full n-saturation.
//!
//! A split `(v, a, b)` branches the state into `v = a` and `v = b`,
//! saturates both branches, and keeps what they agree on. A branch that
//! hits a contradiction is discarded; if both do, the state itself is
//! contradictory.

use std::fmt;
use std::marker::PhantomData;

use crate::{LogicState, SaturationStatus};

/// The variable to branch on and the two values to assume for it.
pub type Split<S> =
    (<S as LogicState>::Var, <S as LogicState>::Val, <S as LogicState>::Val);

/// The result of a single dilemma step.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum StrategicOutcome<S> {
    /// A split derived new facts; this is the strengthened state.
    Valid(S),
    /// Both branches of a split, or the base state itself, are
    /// contradictory.
    Contradiction,
    /// No split derived anything the base state did not already hold.
    Unchanged,
}

/// Runs the dilemma rule over a [`LogicState`].
pub struct DilemmaProcessor<S: LogicState> {
    _marker: PhantomData<S>,
}

impl<S: LogicState> fmt::Debug for DilemmaProcessor<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DilemmaProcessor")
    }
}

impl<S: LogicState + PartialEq> DilemmaProcessor<S> {
    /// Tries each split in order on one thread and returns at the first
    /// one that derives new facts.
    ///
    /// The base state is 0-saturated first. Each branch is 0-saturated
    /// after its assumption.
    pub fn process_sequential<I>(
        base_state: S,
        splits: I,
    ) -> StrategicOutcome<S>
    where
        I: IntoIterator<Item = Split<S>>,
    {
        Self::process_with::<Sequential, I>(base_state, splits)
    }

    /// Like [`process_sequential`](Self::process_sequential), but runs
    /// the two branches of each split on separate rayon workers.
    #[cfg(feature = "rayon")]
    pub fn process_parallel<I>(base_state: S, splits: I) -> StrategicOutcome<S>
    where
        I: IntoIterator<Item = Split<S>>,
    {
        Self::process_with::<Parallel, I>(base_state, splits)
    }

    /// Saturates `state` to a fixpoint under the dilemma rule of the given
    /// depth, on one thread.
    ///
    /// Depth 0 runs [`LogicState::saturate_zero`] until it converges.
    /// Depth `k` repeatedly applies every split, saturating each branch to
    /// depth `k - 1`, until no split derives anything new. Cost grows
    /// exponentially with `depth`.
    ///
    /// # Errors
    ///
    /// Returns the contradiction if `state` is refuted at this depth.
    pub fn saturate(
        state: S,
        depth: usize,
        splits: &[Split<S>],
    ) -> Result<S, S::Error> {
        Self::saturate_with::<Sequential>(state, depth, splits)
    }

    /// Like [`saturate`](Self::saturate), but explores the two branches of
    /// every split with `rayon::join`.
    ///
    /// # Errors
    ///
    /// Returns the contradiction if `state` is refuted at this depth.
    #[cfg(feature = "rayon")]
    pub fn saturate_parallel(
        state: S,
        depth: usize,
        splits: &[Split<S>],
    ) -> Result<S, S::Error> {
        Self::saturate_with::<Parallel>(state, depth, splits)
    }

    fn process_with<F: Fork, I>(
        mut base_state: S,
        splits: I,
    ) -> StrategicOutcome<S>
    where
        I: IntoIterator<Item = Split<S>>,
    {
        if zero_saturate(&mut base_state).is_err() {
            return StrategicOutcome::Contradiction;
        }
        for (var, a, b) in splits {
            let (left, right) = F::join(
                || Self::branch::<F>(&base_state, var, a, 0, &[]),
                || Self::branch::<F>(&base_state, var, b, 0, &[]),
            );
            match merge(&base_state, left, right) {
                Err(_) => return StrategicOutcome::Contradiction,
                Ok(Some(state)) => return StrategicOutcome::Valid(state),
                Ok(None) => {}
            }
        }
        StrategicOutcome::Unchanged
    }

    fn saturate_with<F: Fork>(
        mut state: S,
        depth: usize,
        splits: &[Split<S>],
    ) -> Result<S, S::Error> {
        zero_saturate(&mut state)?;
        let Some(child_depth) = depth.checked_sub(1) else {
            return Ok(state);
        };
        loop {
            let mut changed = false;
            for (var, a, b) in splits {
                let (left, right) = F::join(
                    || {
                        Self::branch::<F>(
                            &state,
                            *var,
                            a.clone(),
                            child_depth,
                            splits,
                        )
                    },
                    || {
                        Self::branch::<F>(
                            &state,
                            *var,
                            b.clone(),
                            child_depth,
                            splits,
                        )
                    },
                );
                if let Some(mut next) = merge(&state, left, right)? {
                    zero_saturate(&mut next)?;
                    state = next;
                    changed = true;
                }
            }
            if !changed {
                return Ok(state);
            }
        }
    }

    fn branch<F: Fork>(
        state: &S,
        var: S::Var,
        val: S::Val,
        depth: usize,
        splits: &[Split<S>],
    ) -> Result<S, S::Error> {
        let mut state = state.clone();
        state.assume(var, val)?;
        Self::saturate_with::<F>(state, depth, splits)
    }
}

/// Runs [`LogicState::saturate_zero`] until it reports
/// [`SaturationStatus::Converged`].
///
/// # Errors
///
/// Returns the contradiction if any pass derives one.
pub fn zero_saturate<S: LogicState>(state: &mut S) -> Result<(), S::Error> {
    while state.saturate_zero()? == SaturationStatus::Changed {}
    Ok(())
}

/// Combines the two branches of a split.
///
/// Returns `Err` when both branches are contradictory, `Ok(Some(_))` with
/// the strengthened state when the split derived something, and
/// `Ok(None)` when it derived nothing beyond `base`.
fn merge<S: LogicState + PartialEq>(
    base: &S,
    left: Result<S, S::Error>,
    right: Result<S, S::Error>,
) -> Result<Option<S>, S::Error> {
    let merged = match (left, right) {
        (Err(_), Err(error)) => return Err(error),
        (Err(_), Ok(state)) | (Ok(state), Err(_)) => state,
        (Ok(left), Ok(right)) => left.intersect(&right),
    };
    Ok((merged != *base).then_some(merged))
}

/// How the two branches of a split are scheduled.
trait Fork {
    fn join<A, B, RA, RB>(a: A, b: B) -> (RA, RB)
    where
        A: FnOnce() -> RA + Send,
        B: FnOnce() -> RB + Send,
        RA: Send,
        RB: Send;
}

/// Runs both branches on the calling thread.
struct Sequential;

impl Fork for Sequential {
    fn join<A, B, RA, RB>(a: A, b: B) -> (RA, RB)
    where
        A: FnOnce() -> RA + Send,
        B: FnOnce() -> RB + Send,
        RA: Send,
        RB: Send,
    {
        (a(), b())
    }
}

/// Runs the branches on rayon's work-stealing pool.
#[cfg(feature = "rayon")]
struct Parallel;

#[cfg(feature = "rayon")]
impl Fork for Parallel {
    fn join<A, B, RA, RB>(a: A, b: B) -> (RA, RB)
    where
        A: FnOnce() -> RA + Send,
        B: FnOnce() -> RB + Send,
        RA: Send,
        RB: Send,
    {
        rayon::join(a, b)
    }
}
