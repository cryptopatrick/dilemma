//! Derives `x1` from `(x1 | x2) & (x1 | !x2)` with one dilemma split.

#[allow(dead_code, reason = "shared with the integration tests")]
#[path = "../tests/cnf.rs"]
mod cnf;

use cnf::Cnf;
use dilemma::engine::{DilemmaProcessor, StrategicOutcome};

fn main() {
    let formula = Cnf::new(2, vec![vec![1, 2], vec![1, -2]]);
    println!("x1 before: {:?}", formula.value(1));

    let outcome =
        DilemmaProcessor::process_sequential(formula, [(2, true, false)]);
    match outcome {
        StrategicOutcome::Valid(state) => {
            println!("x1 after:  {:?}", state.value(1));
        }
        StrategicOutcome::Contradiction => println!("unsatisfiable"),
        StrategicOutcome::Unchanged => println!("nothing derived"),
    }
}
