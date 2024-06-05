#![doc = include_str!("../README.md")]

/// Shared module for internal logic!
/// Contains helper for command aliasing.
pub mod aliasing;
/// Pueue lib's own Error implementation.
pub mod error;
/// Helper classes to read and write log files of Pueue's tasks.
pub mod log;
pub mod network;
/// Shared module for internal logic!
/// Contains helper to spawn shell commands and examine and interact with processes.
pub mod process_helper;
/// This module contains all platform unspecific default values and helper functions for working
/// with our setting representation.
mod setting_defaults;
/// Pueue's configuration representation.
pub mod settings;
/// The main struct used to represent the daemon's current state.
pub mod state;
/// Everything regarding Pueue's task
pub mod task;
