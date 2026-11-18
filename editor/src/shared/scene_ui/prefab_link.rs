/// Identifies whether a linked prefab reference came from a root or child node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefabLinkSource {
    /// The entity is the linked prefab root.
    Root,
    /// The entity is a linked prefab child node.
    Node,
}
