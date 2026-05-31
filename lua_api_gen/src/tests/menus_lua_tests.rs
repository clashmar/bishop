use engine_core::scripting::menus_lua::generate_menus_lua;
use engine_core::menu::{
    ButtonElement, LayoutChild, LayoutConfig, LayoutGroupElement, MenuAction, MenuBackground,
    MenuElement, MenuElementKind, MenuMode, MenuTemplate,
};
use engine_core::prelude::Rect;
use engine_core::scripting::lua_constants::lua_ownership;

fn button(name: &str) -> MenuElement {
    MenuElement {
        name: name.to_string(),
        kind: MenuElementKind::Button(ButtonElement {
            text_key: name.to_string(),
            action: MenuAction::CloseMenu,
            ..Default::default()
        }),
        rect: Rect::new(0.0, 0.0, 1.0, 1.0),
        enabled: true,
        visible: true,
        z_order: 0,
        class: None,
        style_id: None,
    }
}

#[test]
fn generate_menus_lua_emits_ids_and_nested_named_elements() {
    let template = MenuTemplate {
        id: "title".to_string(),
        background: MenuBackground::None,
        elements: vec![MenuElement {
            name: String::new(),
            kind: MenuElementKind::LayoutGroup(LayoutGroupElement {
                layout: LayoutConfig::default(),
                children: vec![
                    LayoutChild { element: button("NewGame"), managed: true },
                    LayoutChild { element: button("LoadGame"), managed: true },
                ],
                nav_targets: Default::default(),
            }),
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            enabled: true,
            visible: true,
            z_order: 0,
            class: None,
            style_id: None,
        }],
        mode: MenuMode::FrontEnd,
        on_open: String::new(),
    };

    let lua = generate_menus_lua(&[template]);

    assert!(lua.contains("Title = {"));
    assert!(lua.contains("Id = \"title\""));
    assert!(lua.contains("NewGame = \"NewGame\""));
    assert!(lua.contains("LoadGame = \"LoadGame\""));
    assert!(lua.contains(lua_ownership::LUA_OWNER_GAME_GENERATED));
}

#[test]
fn generate_menus_lua_sanitizes_duplicate_keys() {
    let template = MenuTemplate {
        id: "pause-menu".to_string(),
        background: MenuBackground::None,
        elements: vec![button("Save Game"), button("Save-Game")],
        mode: MenuMode::Paused,
        on_open: String::new(),
    };

    let lua = generate_menus_lua(&[template]);

    assert!(lua.contains("PauseMenu = {"));
    assert!(lua.contains("SaveGame = \"Save Game\""));
    assert!(lua.contains("SaveGame_2 = \"Save-Game\""));
}

#[test]
fn generate_menus_lua_prefixes_digit_leading_keys() {
    let template = MenuTemplate {
        id: "1up".to_string(),
        background: MenuBackground::None,
        elements: vec![],
        mode: MenuMode::Paused,
        on_open: String::new(),
    };

    let lua = generate_menus_lua(&[template]);

    assert!(lua.contains("Menu1up = {"));
    assert!(lua.contains("Id = \"1up\""));
}
