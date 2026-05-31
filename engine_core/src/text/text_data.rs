use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How text variants are selected when displaying text.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SelectionMode {
    /// Pick any variant randomly each time (can repeat).
    #[default]
    Random,
    /// Show variants in order, one per interaction (0, 1, 2, ...).
    Sequential,
    /// Like sequential, but after all variants shown, display exhausted text forever.
    Once,
    /// Like a shuffled deck, all variants shown once before reshuffling.
    Shuffle,
}

/// A single text entry containing variants and selection behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEntry {
    /// How to select which variant to display.
    #[serde(default)]
    pub selection: SelectionMode,
    /// Text shown when all variants have been exhausted (for "once" mode).
    #[serde(default)]
    pub exhausted: Option<String>,
    /// List of text variants to choose from.
    #[serde(default)]
    pub variants: Vec<String>,
}

impl Default for TextEntry {
    fn default() -> Self {
        Self {
            selection: SelectionMode::Random,
            exhausted: None,
            variants: Vec::new(),
        }
    }
}

/// A text file containing multiple keyed entries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextFile {
    /// Map of entry keys to their text entries.
    #[serde(flatten)]
    pub entries: HashMap<String, TextEntry>,
}

/// A flat key-value text file for UI elements (menus, buttons, labels).
pub type UiTextFile = HashMap<String, String>;

/// Language manifest file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextManifest {
    /// Default language code (e.g., "en").
    pub default_language: String,
    /// List of available language codes.
    pub available: Vec<String>,
}

impl Default for TextManifest {
    fn default() -> Self {
        Self {
            default_language: "en".to_string(),
            available: vec!["en".to_string()],
        }
    }
}

/// State tracking for sequential and shuffle selection modes.
#[derive(Debug, Clone, Default)]
pub struct TextState {
    /// Current index for sequential mode.
    pub index: usize,
    /// Whether all variants have been shown (for "once" mode).
    pub exhausted: bool,
    /// Shuffled order for shuffle mode.
    pub shuffle_order: Vec<usize>,
    /// Current position in shuffle order.
    pub shuffle_index: usize,
}

impl TextState {
    /// Resets the state to initial values.
    pub fn reset(&mut self) {
        self.index = 0;
        self.exhausted = false;
        self.shuffle_order.clear();
        self.shuffle_index = 0;
    }
}
