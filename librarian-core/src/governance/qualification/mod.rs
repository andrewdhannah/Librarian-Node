//! # Model Qualification Consumer
//!
//! Maps existing model qualification to the governance substrate.
//! No new governance primitives — consumes Capability, Evidence, Receipt, ResidencyState.
//!
//! This module validates the architectural thesis that model qualification
//! is a consumer of the existing governance layer, not a feature requiring
//! new primitives.

pub mod profiles;
pub mod harness;
