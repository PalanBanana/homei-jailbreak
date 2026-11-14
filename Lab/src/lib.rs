//! HOMEI virtual research lab: the host controller.
//!
//! This crate is separate from `homei-core` on purpose. Core is portable and contains no file or
//! process code, so it can be embedded in the iOS application. This crate writes to a guest
//! workspace and starts a small allowlist of programs, and it does so only through `guard` and
//! `process`. It never touches a physical device, the network or `sudo`.

pub mod analysis;
pub mod backend;
pub mod bootstrap;
pub mod experiments;
pub mod feasibility;
pub mod firmware;
pub mod ghidra;
pub mod guard;
pub mod hash;
pub mod host;
pub mod packages;
pub mod process;
pub mod state;
pub mod storage;
pub mod timeutil;
