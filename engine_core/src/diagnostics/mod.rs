// engine_core/src/diagnostics/mod.rs
//! Diagnostics infrastructure for engine metrics and performance monitoring.

pub mod collector;
pub mod metrics;

pub use collector::*;
pub use metrics::*;
