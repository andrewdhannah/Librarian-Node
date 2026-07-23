//! # Linux Runtime Adapter
//!
//! Linux implementation of the `RuntimeAdapter` trait.
//! Translates Linux-specific process events (systemd, D-Bus, /proc)
//! into the platform-agnostic `ProcessEvent` type.
//!
//! No Linux-specific governance types are introduced. The adapter
//! terminates at the `RuntimeAdapter` trait boundary.

pub mod adapter;
pub mod service;
