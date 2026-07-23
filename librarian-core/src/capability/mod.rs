//! Capability manifest and Owner decision system.
//!
//! The capability layer bridges qualification evidence and router eligibility.
//! It is the ONLY authority gate between evidence and approval.
//!
//! Authority rules:
//! - evidence exists ≠ role approved
//! - classifier recommends ≠ role approved
//! - Owner decision recorded → manifest may become approved
//!
//! The capability module does NOT:
//! - Auto-promote from evidence to approval
//! - Infer router eligibility
//! - Mutate router selection policy
//!
//! The capability module DOES:
//! - Aggregate sealed qualification evidence
//! - Describe demonstrated primitive and role evidence
//! - Record known failure modes
//! - Propose role status and bounded constraints
//! - Create draft capability manifests
//! - Record explicit Owner decisions
//! - Seal manifests only from Owner decisions

pub mod decisions;
pub mod manifest;
