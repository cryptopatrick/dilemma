//! Integration tests for the dilemma engine over a toy CNF domain.

mod cnf;

use cnf::Cnf;
use dilemma::engine::{
    DilemmaProcessor, Split, StrategicOutcome, zero_saturate,
};

type Processor = DilemmaProcessor<Cnf>;

fn splits_over(variables: u32) -> Vec<Split<Cnf>> {
    (1..=variables).map(|v| (v, true, false)).collect()
}

/// `(x1 | x2) & (x1 | !x2)`: both branches on x2 force x1.
fn forces_x1() -> Cnf {
    Cnf::new(2, vec![vec![1, 2], vec![1, -2]])
}

/// Every clause over x1, x2: refuted by 1-saturation.
fn unsat_two() -> Cnf {
    Cnf::new(2, vec![vec![1, 2], vec![1, -2], vec![-1, 2], vec![-1, -2]])
}

/// Every clause over x1, x2, x3: needs 2-saturation to refute.
fn unsat_three() -> Cnf {
    let mut clauses = Vec::new();
    for signs in 0..8 {
        let lit =
            |bit: i32, var: i32| if signs & bit == 0 { var } else { -var };
        clauses.push(vec![lit(1, 1), lit(2, 2), lit(4, 3)]);
    }
    Cnf::new(3, clauses)
}

#[test]
fn zero_saturation_propagates_units() {
    let mut state = Cnf::new(3, vec![vec![1], vec![-1, 2], vec![-2, 3]]);
    zero_saturate(&mut state).expect("satisfiable");
    assert_eq!(
        [state.value(1), state.value(2), state.value(3)],
        [Some(true); 3]
    );
}

#[test]
fn zero_saturation_reports_conflict() {
    let mut state = Cnf::new(1, vec![vec![1], vec![-1]]);
    assert!(zero_saturate(&mut state).is_err());
}

#[test]
fn step_merges_facts_common_to_both_branches() {
    let StrategicOutcome::Valid(state) =
        Processor::process_sequential(forces_x1(), [(2, true, false)])
    else {
        panic!("expected the split on x2 to derive x1");
    };
    assert_eq!(state.value(1), Some(true));
    assert_eq!(state.value(2), None);
}

#[test]
fn step_adopts_the_surviving_branch() {
    let state = Cnf::new(2, vec![vec![-1, 2], vec![-1, -2]]);
    let outcome = Processor::process_sequential(state, [(1, true, false)]);
    let StrategicOutcome::Valid(state) = outcome else {
        panic!("expected x1 = false, got {outcome:?}");
    };
    assert_eq!(state.value(1), Some(false));
}

#[test]
fn step_reports_contradiction() {
    let outcome = Processor::process_sequential(unsat_two(), splits_over(2));
    assert_eq!(outcome, StrategicOutcome::Contradiction);
}

#[test]
fn step_reports_unchanged() {
    let state = Cnf::new(2, vec![vec![1, 2]]);
    let outcome = Processor::process_sequential(state, splits_over(2));
    assert_eq!(outcome, StrategicOutcome::Unchanged);
}

#[test]
fn saturation_depth_bounds_reasoning_power() {
    let splits = splits_over(3);
    let one = Processor::saturate(unsat_three(), 1, &splits);
    assert!(one.is_ok(), "1-saturation cannot refute: {one:?}");
    let two = Processor::saturate(unsat_three(), 2, &splits);
    assert!(two.is_err(), "2-saturation refutes: {two:?}");
}

#[test]
fn saturation_reaches_fixpoint() {
    let state = Processor::saturate(forces_x1(), 1, &splits_over(2))
        .expect("satisfiable");
    assert_eq!(state.value(1), Some(true));
    let again = Processor::saturate(state.clone(), 1, &splits_over(2))
        .expect("satisfiable");
    assert_eq!(again, state, "saturation is idempotent");
}

#[cfg(feature = "rayon")]
#[test]
fn parallel_agrees_with_sequential() {
    let cases = [(forces_x1(), 2), (unsat_two(), 2), (unsat_three(), 3)];
    for (state, variables) in cases {
        let splits = splits_over(variables);
        assert_eq!(
            Processor::process_parallel(state.clone(), splits.clone()),
            Processor::process_sequential(state.clone(), splits.clone()),
        );
        for depth in 0..=2 {
            assert_eq!(
                Processor::saturate_parallel(state.clone(), depth, &splits),
                Processor::saturate(state.clone(), depth, &splits),
                "depth {depth}, state {state:?}"
            );
        }
    }
}

#[test]
fn splitting_a_decided_variable_derives_nothing() {
    let state = Cnf::new(1, vec![vec![1]]);
    let outcome = Processor::process_sequential(state, splits_over(1));
    assert_eq!(outcome, StrategicOutcome::Unchanged);
}
