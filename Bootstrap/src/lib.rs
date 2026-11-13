//! HOMEI bootstrap planning and simulation.
//!
//! Simulation only. This crate computes an ordering of synthetic components and prints what
//! would happen. It does not touch the filesystem, run processes, open network connections or
//! reach any device, and it reports `writes_performed = 0` as a fact, not a promise.

pub mod models;
pub mod planning;
pub mod simulation;
