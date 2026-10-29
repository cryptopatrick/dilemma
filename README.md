# dilemma

**A generic engine for the dilemma rule of Stålmarck's method.**

[![Crates.io](https://img.shields.io/crates/v/dilemma.svg)](https://crates.io/crates/dilemma)
[![Documentation](https://docs.rs/dilemma/badge.svg)](https://docs.rs/dilemma)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](https://github.com/cryptopatrick/dilemma)

## Overview

`dilemma` is not a SAT solver. It is the branch-and-merge core of
Stålmarck's method, made generic so that constraint solvers, model
checkers, and theorem provers can reuse it on their own state types.

You describe your logic by implementing [`LogicState`]: how to run one
pass of local (0-saturation) rules, how to assume a value, and how to
intersect two states. The engine then does the rest:

1. Split the state on a variable `v` into branches `v = a` and `v = b`.
2. Saturate both branches.
3. Keep only what both branches agree on. If one branch is
   contradictory, keep the other; if both are, the state is refuted.

Repeating this to a fixpoint is 1-saturation. Saturating each branch to
depth `k - 1` gives k-saturation.

## Key Features

- Static dispatch only: your state type is a generic parameter, never a
  trait object.
- Sequential and parallel drivers; the parallel driver runs the two
  branches of each split with `rayon::join` (feature `rayon`, on by
  default).
- Arbitrary saturation depth through
  [`DilemmaProcessor::saturate`](engine::DilemmaProcessor::saturate).

## Quick Start

```toml
[dependencies]
dilemma = "0.1"
```

A propositional state where 0-saturation is unit propagation:

```rust
use std::fmt;
use dilemma::engine::{DilemmaProcessor, StrategicOutcome};
use dilemma::{LogicState, SaturationStatus};

#[derive(Debug, Clone, PartialEq)]
struct Cnf {
    clauses: Vec<Vec<i32>>,      // DIMACS literals: 2 is x2, -2 is !x2
    values: Vec<Option<bool>>,   // indexed by variable
}

#[derive(Debug)]
struct Conflict;

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("conflict")
    }
}

impl std::error::Error for Conflict {}

impl Cnf {
    fn lit(&self, l: i32) -> Option<bool> {
        self.values[l.unsigned_abs() as usize].map(|v| v == (l > 0))
    }
}

impl LogicState for Cnf {
    type Var = usize;
    type Val = bool;
    type Error = Conflict;

    fn saturate_zero(&mut self) -> Result<SaturationStatus, Conflict> {
        for clause in self.clauses.clone() {
            if clause.iter().any(|&l| self.lit(l) == Some(true)) {
                continue;
            }
            let open: Vec<i32> = clause
                .iter()
                .copied()
                .filter(|&l| self.lit(l).is_none())
                .collect();
            match open[..] {
                [] => return Err(Conflict),
                [unit] => {
                    self.assume(unit.unsigned_abs() as usize, unit > 0)?;
                    return Ok(SaturationStatus::Changed);
                }
                _ => {}
            }
        }
        Ok(SaturationStatus::Converged)
    }

    fn assume(&mut self, var: usize, val: bool) -> Result<(), Conflict> {
        match self.values[var] {
            Some(old) if old != val => Err(Conflict),
            _ => {
                self.values[var] = Some(val);
                Ok(())
            }
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        let values = self
            .values
            .iter()
            .zip(&other.values)
            .map(|(a, b)| if a == b { *a } else { None })
            .collect();
        Self { clauses: self.clauses.clone(), values }
    }
}

# fn main() -> Result<(), Conflict> {
// (x1 | x2) & (x1 | !x2): neither clause is a unit, but both
// branches of a split on x2 force x1.
let formula = Cnf {
    clauses: vec![vec![1, 2], vec![1, -2]],
    values: vec![None; 3],
};

let outcome =
    DilemmaProcessor::process_sequential(formula.clone(), [(2, true, false)]);
let StrategicOutcome::Valid(state) = outcome else { unreachable!() };
assert_eq!(state.values[1], Some(true));

// Or saturate to a fixpoint at depth 1 over every variable.
let splits = [(1, true, false), (2, true, false)];
let state = DilemmaProcessor::saturate(formula, 1, &splits)?;
assert_eq!(state.values[1], Some(true));
# Ok(())
# }
```

A fuller version lives in [`tests/cnf.rs`](https://github.com/cryptopatrick/dilemma/blob/master/tests/cnf.rs);
run it with `cargo run --example basic`.

## Roadmap

- Variable-selection heuristics (degree, activity).
- Persistent, structurally shared state wrappers and benchmarks.
- LRAT proof-certificate output.

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](https://github.com/cryptopatrick/dilemma/blob/master/CONTRIBUTING.md).

## License

Licensed under the GNU Affero General Public License v3.0 only
(AGPL-3.0-only). See [LICENSE](https://github.com/cryptopatrick/dilemma/blob/master/LICENSE).
