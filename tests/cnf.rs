//! A minimal CNF domain: unit propagation is the 0-saturation rule.
//!
//! Shared by the integration tests and the `basic` example.

use std::fmt;
use std::sync::Arc;

use dilemma::{LogicState, SaturationStatus};

/// A CNF formula over variables `1..=n` with a partial assignment.
///
/// Literals use DIMACS convention: `3` is `x3`, `-3` is `!x3`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cnf {
    clauses: Arc<[Vec<i32>]>,
    assignment: Vec<Option<bool>>,
}

impl Cnf {
    pub fn new(variables: u32, clauses: Vec<Vec<i32>>) -> Self {
        let slots = usize::try_from(variables).expect("fits in usize") + 1;
        Self { clauses: clauses.into(), assignment: vec![None; slots] }
    }

    pub fn value(&self, variable: u32) -> Option<bool> {
        self.assignment[slot(variable)]
    }

    fn literal(&self, literal: i32) -> Option<bool> {
        self.value(literal.unsigned_abs()).map(|v| v == (literal > 0))
    }
}

/// Two unit propagations, or an assumption, disagreed about a variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub variable: u32,
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "conflicting values for x{}", self.variable)
    }
}

impl std::error::Error for Conflict {}

impl LogicState for Cnf {
    type Var = u32;
    type Val = bool;
    type Error = Conflict;

    fn saturate_zero(&mut self) -> Result<SaturationStatus, Conflict> {
        let mut status = SaturationStatus::Converged;
        for index in 0..self.clauses.len() {
            let clause = &self.clauses[index];
            if clause.iter().any(|&l| self.literal(l) == Some(true)) {
                continue;
            }
            let mut open =
                clause.iter().filter(|&&l| self.literal(l).is_none());
            match (open.next().copied(), open.next()) {
                (None, _) => {
                    let variable = clause[0].unsigned_abs();
                    return Err(Conflict { variable });
                }
                (Some(unit), None) => {
                    self.assume(unit.unsigned_abs(), unit > 0)?;
                    status = SaturationStatus::Changed;
                }
                (Some(_), Some(_)) => {}
            }
        }
        Ok(status)
    }

    fn assume(&mut self, variable: u32, value: bool) -> Result<(), Conflict> {
        let current = &mut self.assignment[slot(variable)];
        match *current {
            Some(existing) if existing != value => Err(Conflict { variable }),
            _ => {
                *current = Some(value);
                Ok(())
            }
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        let assignment = self
            .assignment
            .iter()
            .zip(&other.assignment)
            .map(|(a, b)| if a == b { *a } else { None })
            .collect();
        Self { clauses: Arc::clone(&self.clauses), assignment }
    }
}

fn slot(variable: u32) -> usize {
    usize::try_from(variable).expect("variable index fits in usize")
}
