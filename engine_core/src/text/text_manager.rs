// engine_core/src/text/text_manager.rs
use crate::text::*;
use rand::Rng;
use rand::seq::SliceRandom;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Manages text loading, caching, and text selection.
pub struct TextManager {
    /// Base path for text files.
    text_root: PathBuf,
    /// Current language code.
    current_language: String,
    /// Available languages from manifest.
    available_languages: Vec<String>,
    /// Cached text files by text_id.
    cache: RefCell<HashMap<String, TextFile>>,
    /// Cached UI text files by text_id.
    ui_cache: RefCell<HashMap<String, UiTextFile>>,
    /// State tracking for each (text_id, key) pair.
    state: RefCell<HashMap<(String, String), TextState>>,
    /// Global dialogue configuration.
    pub config: DialogueConfig,
}

impl Default for TextManager {
    fn default() -> Self {
        Self {
            text_root: PathBuf::new(),
            current_language: "en".to_string(),
            available_languages: vec!["en".to_string()],
            cache: RefCell::new(HashMap::new()),
            ui_cache: RefCell::new(HashMap::new()),
            state: RefCell::new(HashMap::new()),
            config: DialogueConfig::default(),
        }
    }
}

impl TextManager {
    /// Creates a new TextManager with the given root path.
    pub fn new(text_root: PathBuf) -> Self {
        let mut manager = Self {
            text_root,
            ..Default::default()
        };
        manager.load_manifest();
        manager
    }

    /// Loads the manifest file to get available languages.
    fn load_manifest(&mut self) {
        let manifest_path = self.text_root.join("_manifest.toml");
        if let Ok(content) = fs::read_to_string(&manifest_path)
            && let Ok(manifest) = toml::from_str::<TextManifest>(&content)
        {
            self.current_language = manifest.default_language;
            self.available_languages = manifest.available;
        }
    }

    /// Sets the current language. Returns false if language is not available.
    pub fn set_language(&mut self, lang: &str) -> bool {
        if self.available_languages.contains(&lang.to_string()) {
            self.current_language = lang.to_string();
            self.cache.borrow_mut().clear();
            self.ui_cache.borrow_mut().clear();
            true
        } else {
            false
        }
    }

    /// Returns the current language code.
    pub fn get_language(&self) -> &str {
        &self.current_language
    }

    /// Returns a list of available languages.
    pub fn get_languages(&self) -> &[String] {
        &self.available_languages
    }

    /// Loads a text file by ID, supporting subfolders (e.g., "dialogue/npcs/npc" -> "dialogue/npcs/npc.toml").
    fn load_text_file(&self, text_id: &str) -> bool {
        if self.cache.borrow().contains_key(text_id) {
            return true;
        }

        // Build path from text_id, supporting subfolders (e.g., "dialogue/npcs/npc")
        let normalized_id = text_id.replace('\\', "/");
        let mut file_path = self.text_root.join(&self.current_language);
        for component in normalized_id.split('/') {
            file_path = file_path.join(component);
        }
        file_path.set_extension("toml");

        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to load text file '{}': {e}", file_path.display());
                return false;
            }
        };

        let text_file: TextFile = match toml::from_str(&content) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("Failed to parse text file '{}': {e}", file_path.display());
                return false;
            }
        };

        self.cache
            .borrow_mut()
            .insert(text_id.to_string(), text_file);
        true
    }

    /// Selects and returns text for the given text_id and key.
    pub fn select_text(&self, text_id: &str, key: &str) -> Option<String> {
        if !self.load_text_file(text_id) {
            return None;
        }

        let cache = self.cache.borrow();
        let file = cache.get(text_id)?;
        let entry = file.entries.get(key)?.clone();
        drop(cache);

        if entry.variants.is_empty() {
            return entry.exhausted.clone();
        }

        let state_key = (text_id.to_string(), key.to_string());
        let mut state_map = self.state.borrow_mut();
        let state = state_map.entry(state_key).or_default();

        Self::select_from_entry_static(&entry, state)
    }

    /// Selects text from an entry based on its selection mode.
    fn select_from_entry_static(entry: &TextEntry, state: &mut TextState) -> Option<String> {
        if entry.variants.is_empty() {
            return entry.exhausted.clone();
        }

        match entry.selection {
            SelectionMode::Random => {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..entry.variants.len());
                Some(entry.variants[idx].clone())
            }
            SelectionMode::Sequential => {
                if entry.exhausted.is_some() {
                    if state.exhausted {
                        return entry.exhausted.clone();
                    }
                    let text = entry.variants.get(state.index).cloned();
                    state.index += 1;
                    if state.index >= entry.variants.len() {
                        state.exhausted = true;
                    }
                    text.or_else(|| entry.exhausted.clone())
                } else {
                    let text = entry.variants.get(state.index).cloned();
                    state.index = (state.index + 1) % entry.variants.len();
                    text
                }
            }
            SelectionMode::Once => {
                if state.exhausted {
                    return entry.exhausted.clone();
                }
                let text = entry.variants.get(state.index).cloned();
                state.index += 1;
                if state.index >= entry.variants.len() {
                    state.exhausted = true;
                }
                text.or_else(|| entry.exhausted.clone())
            }
            SelectionMode::Shuffle => {
                if entry.exhausted.is_some() && state.exhausted {
                    return entry.exhausted.clone();
                }

                if state.shuffle_order.is_empty()
                    || (entry.exhausted.is_none()
                        && state.shuffle_index >= state.shuffle_order.len())
                {
                    let mut rng = rand::thread_rng();
                    state.shuffle_order = (0..entry.variants.len()).collect();
                    state.shuffle_order.shuffle(&mut rng);
                    state.shuffle_index = 0;
                }

                if state.shuffle_index >= state.shuffle_order.len() {
                    state.exhausted = true;
                    return entry.exhausted.clone();
                }

                let idx = state.shuffle_order[state.shuffle_index];
                state.shuffle_index += 1;

                if entry.exhausted.is_some() && state.shuffle_index >= state.shuffle_order.len() {
                    state.exhausted = true;
                }

                entry.variants.get(idx).cloned()
            }
        }
    }

    /// Resets state for a specific text entry.
    pub fn reset_state(&self, text_id: &str, key: &str) {
        let state_key = (text_id.to_string(), key.to_string());
        if let Some(state) = self.state.borrow_mut().get_mut(&state_key) {
            state.reset();
        }
    }

    /// Clears all cached text files.
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
        self.ui_cache.borrow_mut().clear();
    }

    /// Updates the text root path.
    pub fn set_text_root(&mut self, root: PathBuf) {
        self.text_root = root;
        self.cache.borrow_mut().clear();
        self.ui_cache.borrow_mut().clear();
        self.state.borrow_mut().clear();
        self.load_manifest();
    }

    /// Loads a UI text file by ID into the cache.
    fn load_ui_text_file(&self, text_id: &str) -> bool {
        if self.ui_cache.borrow().contains_key(text_id) {
            return true;
        }

        let normalized_id = text_id.replace('\\', "/");
        let mut file_path = self.text_root.join(&self.current_language);
        for component in normalized_id.split('/') {
            file_path = file_path.join(component);
        }
        file_path.set_extension("toml");

        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to load UI text file '{}': {e}", file_path.display());
                return false;
            }
        };

        let ui_file: UiTextFile = match toml::from_str(&content) {
            Ok(f) => f,
            Err(e) => {
                log::warn!(
                    "Failed to parse UI text file '{}': {e}",
                    file_path.display()
                );
                return false;
            }
        };

        self.ui_cache
            .borrow_mut()
            .insert(text_id.to_string(), ui_file);
        true
    }

    /// Returns UI text for the given text_id and key.
    pub fn get_ui_text(&self, text_id: &str, key: &str) -> Option<String> {
        if !self.load_ui_text_file(text_id) {
            return None;
        }

        let cache = self.ui_cache.borrow();
        cache.get(text_id)?.get(key).cloned()
    }

    /// Resolves UI text for the given text_id and key, falling back to the key itself.
    pub fn resolve_ui_text(&self, text_id: &str, key: &str) -> String {
        self.get_ui_text(text_id, key)
            .unwrap_or_else(|| key.to_string())
    }

    /// Returns the current text root path.
    pub fn get_text_root(&self) -> &Path {
        &self.text_root
    }
}
