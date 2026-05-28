use engine_core::menu::{LayoutChild, MenuElement, MenuElementKind, MenuTemplate};
use engine_core::scripting::lua_constants::lua_ownership;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub fn generate_menus_lua_from_dir(menus_dir: &Path) -> Result<String, String> {
    let mut templates = Vec::new();
    for entry in fs::read_dir(menus_dir)
        .map_err(|e| format!("Failed to read menus directory '{}': {e}", menus_dir.display()))?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "ron") {
            continue;
        }
        let ron = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read menu file '{}': {e}", path.display()))?;
        let template = ron::from_str::<MenuTemplate>(&ron)
            .map_err(|e| format!("Failed to parse menu file '{}': {e}", path.display()))?;
        templates.push(template);
    }
    templates.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(generate_menus_lua(&templates))
}

pub fn generate_menus_lua(templates: &[MenuTemplate]) -> String {
    let mut out = format!(
        "-- Auto-generated. Do not edit.\n{}\n---@meta\n\n---@class Menus\nlocal Menu = {{\n",
        lua_ownership::LUA_OWNER_GAME_GENERATED,
    );

    let mut used_menu_keys = HashSet::new();
    for template in templates {
        let menu_key = pascal_key(&template.id, "Menu", &mut used_menu_keys);
        out.push_str(&format!("    {} = {{\n", menu_key));
        out.push_str(&format!("        Id = {},\n", escape(&template.id)));

        let names = collect_names(&template.elements);
        let mut used_element_keys = HashSet::new();
        for name in &names {
            let key = pascal_key(name, "Element", &mut used_element_keys);
            out.push_str(&format!("        {} = {},\n", key, escape(name)));
        }

        out.push_str("    },\n");
    }

    out.push_str("}\n\nreturn Menu\n");
    out
}

fn collect_names(elements: &[MenuElement]) -> Vec<String> {
    let mut names = Vec::new();
    collect_names_recursive(elements, &mut names);
    names
}

fn collect_names_recursive(elements: &[MenuElement], out: &mut Vec<String>) {
    for element in elements {
        if !element.name.trim().is_empty() {
            out.push(element.name.clone());
        }
        if let MenuElementKind::LayoutGroup(group) = &element.kind {
            collect_children_names(&group.children, out);
        }
    }
}

fn collect_children_names(children: &[LayoutChild], out: &mut Vec<String>) {
    for child in children {
        if !child.element.name.trim().is_empty() {
            out.push(child.element.name.clone());
        }
        if let MenuElementKind::LayoutGroup(group) = &child.element.kind {
            collect_children_names(&group.children, out);
        }
    }
}

fn pascal_key(value: &str, prefix: &str, used: &mut HashSet<String>) -> String {
    let sanitized = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>();
    let trimmed = sanitized
        .trim_matches('_')
        .replace("__", "_");
    let trimmed = trimmed.trim_matches('_');

    let mut result = String::new();
    let mut upper = true;
    for c in trimmed.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            result.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            result.push(c);
        }
    }

    let needs_prefix = result.is_empty() || result.chars().next().is_some_and(|c| c.is_ascii_digit());
    let base = if needs_prefix { format!("{prefix}{result}") } else { result };

    let mut deduped = base.clone();
    let mut suffix = 2;
    while !used.insert(deduped.clone()) {
        deduped = format!("{base}_{suffix}");
        suffix += 1;
    }
    deduped
}

fn escape(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
