#![doc = include_str!("../README.md")]

pub mod engine;
mod state;

pub use state::{LogicState, LogicValue, LogicVariable, SaturationStatus};
