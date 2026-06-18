//! Diagnostics infrastructure for engine metrics and performance monitoring.

pub mod collector;
pub mod metrics;
pub mod residency;

pub use collector::*;
pub use metrics::*;
pub use residency::*;
