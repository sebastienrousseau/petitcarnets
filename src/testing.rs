//! Test helpers: a scripted terminal for running a model headless.

pub use crate::backend::{Screen, TestBackend};

#[cfg(test)]
pub(crate) mod vt;
