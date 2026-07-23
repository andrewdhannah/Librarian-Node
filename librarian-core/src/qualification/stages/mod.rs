//! Qualification stages — progressive testing stages.
//!
//! Each stage builds on the prior stage's evidence:
//! - Stage 1 (Smoke): Runtime works at all — model loads, generates, releases.
//! - Stage 2 (Primitive Probe): Output meets task-specific quality criteria.
//! - Future stages: Role-specific qualification trials.
//!
//! No stage result implies capability or router eligibility.

pub mod primitive_probes;
pub mod smoke;
