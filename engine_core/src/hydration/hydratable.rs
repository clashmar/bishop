use std::fmt;
use std::hash::Hash;

/// Why an asset could not be evicted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvictError {
    /// Ref-count still above zero.
    StillReferenced { count: usize },
    /// Domain-specific keep-alive (e.g. live script instances).
    HasLiveConsumers,
}

impl fmt::Display for EvictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvictError::StillReferenced { count } => {
                write!(f, "asset still referenced (ref-count: {count})")
            }
            EvictError::HasLiveConsumers => {
                write!(f, "asset has live consumers preventing eviction")
            }
        }
    }
}

/// Ref-counted residency contract for asset managers.
///
/// Eviction succeeds only when the ref-count reaches zero and no
/// domain-specific keep-alive conditions exist.
pub trait Hydratable {
    /// The asset identifier type.
    type Id: Clone + Eq + Hash;

    /// Number of active references to this asset.
    fn ref_count(&self, id: &Self::Id) -> usize;

    /// Increment the reference count.
    fn increment_ref(&mut self, id: Self::Id);

    /// Decrement the reference count.
    fn decrement_ref(&mut self, id: Self::Id);

    /// Attempt eviction. Returns `Err` if the asset cannot be removed.
    fn evict(&mut self, id: &Self::Id) -> Result<(), EvictError>;
}
