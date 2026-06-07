use serde::{Deserialize, Serialize};
use std::fmt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::scripting::helpers::sanitize_lua_identifier;
use crate::scripting::lua_constants::{lua_event_tag, lua_ownership};
use std::collections::BTreeSet;

/// Identifies metadata passed through events.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum EventTag {
    /// Triggers autosave on room entry.
    Autosave,
    /// User-defined tag identified by name.
    Custom(String),
}

impl EventTag {
    /// The canonical Lua name, used in generated event_tags.lua.
    pub fn lua_name(&self) -> &str {
        match self {
            EventTag::Autosave => lua_event_tag::AUTOSAVE,
            EventTag::Custom(name) => name.as_str(),
        }
    }

    /// Editor-facing display label.
    pub fn display_name(&self) -> &str {
        match self {
            EventTag::Autosave => lua_event_tag::AUTOSAVE,
            EventTag::Custom(name) => name.as_str(),
        }
    }

    /// Discriminator string ("Autosave" or "Custom").
    pub fn kind(&self) -> &str {
        match self {
            EventTag::Autosave => lua_event_tag::AUTOSAVE,
            EventTag::Custom(_) => lua_event_tag::CUSTOM,
        }
    }
}

impl fmt::Display for EventTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Iterates engine-defined variants only (excludes Custom).
pub fn builtin_event_tags() -> impl Iterator<Item = EventTag> {
    EventTag::iter().filter(|tag| !matches!(tag, EventTag::Custom(_)))
}

/// Generates the content for event_tags.lua with built-in and custom tags.
pub fn generate_event_tags_lua(custom_tags: &[String]) -> String {
    let mut lua = format!(
        "-- Auto-generated. Do not edit.\n\
        {}\n\
        ---@meta\n\n\
        ---@enum EventTag\n\
        local EventTag = {{\n",
        lua_ownership::LUA_OWNER_GAME_GENERATED,
    );

    let mut builtin_names: BTreeSet<String> = BTreeSet::new();
    for tag in builtin_event_tags() {
        let name = tag.lua_name().to_string();
        builtin_names.insert(name.clone());
        lua.push_str(&format!("    {name} = \"{name}\",\n"));
    }

    let custom_sorted: BTreeSet<String> = custom_tags
        .iter()
        .filter(|c| !builtin_names.contains(*c))
        .cloned()
        .collect();

    for clip in &custom_sorted {
        let key = sanitize_lua_identifier(clip, "Tag");
        lua.push_str(&format!("    {key} = \"{clip}\",\n"));
    }

    lua.push_str("}\n\nreturn EventTag\n");
    lua
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn event_tag_enum_iter_yields_engine_variants() {
        let tags: Vec<EventTag> = EventTag::iter().collect();
        assert!(tags.contains(&EventTag::Autosave));
    }

    #[test]
    fn builtin_event_tags_excludes_custom() {
        let builtins: Vec<EventTag> = builtin_event_tags().collect();
        assert_eq!(builtins, vec![EventTag::Autosave]);
    }

    #[test]
    fn autosave_lua_name_returns_autosave() {
        assert_eq!(EventTag::Autosave.lua_name(), "Autosave");
    }

    #[test]
    fn custom_lua_name_returns_the_inner_string() {
        assert_eq!(EventTag::Custom("MyTag".into()).lua_name(), "MyTag");
    }

    #[test]
    fn autosave_display_name_returns_autosave() {
        assert_eq!(EventTag::Autosave.display_name(), "Autosave");
    }

    #[test]
    fn custom_display_name_returns_the_inner_string() {
        assert_eq!(EventTag::Custom("MyTag".into()).display_name(), "MyTag");
    }

    #[test]
    fn autosave_kind_returns_autosave() {
        assert_eq!(EventTag::Autosave.kind(), "Autosave");
    }

    #[test]
    fn custom_kind_returns_custom() {
        assert_eq!(EventTag::Custom("MyTag".into()).kind(), "Custom");
    }

    #[test]
    fn generate_event_tags_lua_emits_engine_variants_with_no_custom() {
        let lua = generate_event_tags_lua(&[]);
        assert!(lua.contains("Autosave = \"Autosave\""));
        assert!(lua.contains(lua_ownership::LUA_OWNER_GAME_GENERATED));
        assert!(lua.contains("---@enum EventTag"));
        assert!(lua.ends_with("return EventTag\n"));
    }

    #[test]
    fn generate_event_tags_lua_includes_custom_tags() {
        let lua = generate_event_tags_lua(&["MyTag".into()]);
        assert!(lua.contains("Autosave = \"Autosave\""));
        assert!(lua.contains("MyTag = \"MyTag\""));
    }

    #[test]
    fn generate_event_tags_lua_deduplicates_custom_tags() {
        let lua = generate_event_tags_lua(&["MyTag".into(), "MyTag".into(), "OtherTag".into()]);
        let mytag_count = lua.matches("MyTag = \"MyTag\"").count();
        assert_eq!(mytag_count, 1);
        assert!(lua.contains("OtherTag = \"OtherTag\""));
    }

    #[test]
    fn generate_event_tags_lua_sorts_custom_tags_alphabetically() {
        let lua = generate_event_tags_lua(&["Zebra".into(), "Alpha".into()]);
        let alpha_pos = lua.find("Alpha = \"Alpha\"").unwrap();
        let zebra_pos = lua.find("Zebra = \"Zebra\"").unwrap();
        assert!(alpha_pos < zebra_pos);
    }

    #[test]
    fn generate_event_tags_lua_filters_custom_tags_matching_engine_variants() {
        let lua = generate_event_tags_lua(&["Autosave".into()]);
        let autosave_count = lua.matches("Autosave = \"Autosave\"").count();
        assert_eq!(autosave_count, 1);
    }

    #[test]
    fn generate_event_tags_lua_sanitizes_non_alphanumeric_names() {
        let lua = generate_event_tags_lua(&["my-tag!".into(), "123abc".into()]);
        assert!(lua.contains("MyTag = \"my-tag!\""));
        assert!(lua.contains("Tag_123abc = \"123abc\""));
    }
}
