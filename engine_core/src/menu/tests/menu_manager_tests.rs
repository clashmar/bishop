use super::*;

#[test]
fn gameplay_pause_policy_toggles_the_pause_menu_with_pause_key() {
    let mut manager = MenuManager::new();
    manager.set_input_policy(MenuInputPolicy::GameplayPause {
        pause_menu_id: "pause".to_string(),
    });

    manager.apply_input_shortcuts(true, false);
    assert_eq!(manager.active_menu_id(), Some("pause"));

    manager.apply_input_shortcuts(true, false);
    assert_eq!(manager.active_menu_id(), None);
}

#[test]
fn front_end_policy_ignores_pause_key_toggle() {
    let mut manager = MenuManager::new();
    manager.set_input_policy(MenuInputPolicy::FrontEnd);
    manager.open_menu("pause");

    manager.apply_input_shortcuts(true, false);

    assert_eq!(manager.active_menu_id(), Some("pause"));
}

#[test]
fn front_end_policy_only_closes_submenus_on_cancel() {
    let mut manager = MenuManager::new();
    manager.set_input_policy(MenuInputPolicy::FrontEnd);
    manager.open_menu("pause");
    manager.open_menu("settings");

    manager.apply_input_shortcuts(false, true);
    assert_eq!(manager.active_menu_id(), Some("pause"));

    manager.apply_input_shortcuts(false, true);
    assert_eq!(manager.active_menu_id(), Some("pause"));
}

#[test]
fn front_end_mode_hides_game_even_with_non_opaque_background() {
    let mut manager = MenuManager::new();
    manager.register_template(MenuTemplate {
        id: "start".to_string(),
        background: MenuBackground::None,
        elements: Vec::new(),
        mode: MenuMode::FrontEnd,
        on_open: String::new(),
    });

    manager.open_menu("start");

    assert!(manager.is_hiding_game());
}

#[test]
fn hover_sets_focus_on_menu_button() {
    let mut manager = MenuManager::new();
    let template = MenuTemplate {
        id: "test".to_string(),
        background: MenuBackground::None,
        elements: vec![
            MenuElement::button(
                "btn0".to_string(),
                MenuAction::Resume,
                Rect::new(0.0, 0.0, 0.5, 1.0),
            ),
            MenuElement::button(
                "btn1".to_string(),
                MenuAction::CloseMenu,
                Rect::new(0.5, 0.0, 0.5, 1.0),
            ),
        ],
        mode: MenuMode::Paused,
        on_open: String::new(),
    };
    manager.register_template(template.clone());
    manager.open_menu("test");

    manager.handle_mouse_hover(&template, Vec2::new(0.75, 0.5));
    assert_eq!(manager.focus.node, 1);
}

#[test]
fn hover_leaves_focus_stays_on_menu() {
    let mut manager = MenuManager::new();
    let template = MenuTemplate {
        id: "test".to_string(),
        background: MenuBackground::None,
        elements: vec![
            MenuElement::button(
                "btn0".to_string(),
                MenuAction::Resume,
                Rect::new(0.0, 0.0, 0.5, 1.0),
            ),
            MenuElement::button(
                "btn1".to_string(),
                MenuAction::CloseMenu,
                Rect::new(0.5, 0.0, 0.5, 1.0),
            ),
        ],
        mode: MenuMode::Paused,
        on_open: String::new(),
    };
    manager.register_template(template.clone());
    manager.open_menu("test");

    manager.handle_mouse_hover(&template, Vec2::new(0.75, 0.5));
    assert_eq!(manager.focus.node, 1);

    manager.handle_mouse_hover(&template, Vec2::new(-1.0, -1.0));
    assert_eq!(manager.focus.node, 1);
}

#[test]
fn reopen_menu_resets_last_hovered() {
    let mut manager = MenuManager::new();
    let template = MenuTemplate {
        id: "test".to_string(),
        background: MenuBackground::None,
        elements: vec![MenuElement::button(
            "btn0".to_string(),
            MenuAction::Resume,
            Rect::new(0.0, 0.0, 1.0, 1.0),
        )],
        mode: MenuMode::Paused,
        on_open: String::new(),
    };
    manager.register_template(template.clone());

    manager.open_menu("test");
    manager.handle_mouse_hover(&template, Vec2::new(0.5, 0.5));
    manager.close_menu();

    manager.open_menu("test");
    manager.handle_mouse_hover(&template, Vec2::new(0.5, 0.5));
    assert_eq!(manager.focus.node, 0);
}

fn flat_test_template() -> MenuTemplate {
    MenuTemplate {
        id: "test".to_string(),
        background: MenuBackground::None,
        elements: vec![
            MenuElement {
                name: "BtnA".to_string(),
                kind: MenuElementKind::Panel(PanelElement),
                rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                enabled: true,
                visible: true,
                z_order: 0,
                class: None,
                style_id: None,
            },
            MenuElement {
                name: "BtnB".to_string(),
                kind: MenuElementKind::Panel(PanelElement),
                rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                enabled: true,
                visible: true,
                z_order: 0,
                class: None,
                style_id: None,
            },
        ],
        mode: MenuMode::Paused,
        on_open: String::new(),
    }
}

fn named_button(name: &str) -> MenuElement {
    let mut button = MenuElement::button(
        name.to_string(),
        MenuAction::Resume,
        Rect::new(0.0, 0.0, 1.0, 1.0),
    );
    button.name = name.to_string();
    button
}

fn grouped_test_template(on_open: &str) -> MenuTemplate {
    MenuTemplate {
        id: "title".to_string(),
        background: MenuBackground::None,
        elements: vec![MenuElement {
            name: String::new(),
            kind: MenuElementKind::LayoutGroup(LayoutGroupElement {
                layout: LayoutConfig::default(),
                children: vec![
                    LayoutChild {
                        element: named_button("NewGame"),
                        managed: true,
                    },
                    LayoutChild {
                        element: named_button("LoadGame"),
                        managed: true,
                    },
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
        on_open: on_open.to_string(),
    }
}

#[test]
fn set_element_enabled_toggles_found_top_level_element() {
    let mut mgr = MenuManager::new();
    mgr.register_template(flat_test_template());

    assert!(mgr.set_element_enabled("test", "BtnA", false));
    let tmpl = mgr.get_template("test").unwrap();
    assert!(!tmpl.elements.iter().find(|e| e.name == "BtnA").unwrap().enabled);
    assert!(tmpl.elements.iter().find(|e| e.name == "BtnB").unwrap().enabled);
}

#[test]
fn set_element_enabled_updates_named_layout_child() {
    let mut mgr = MenuManager::new();
    mgr.register_template(grouped_test_template(""));

    assert!(mgr.set_element_enabled("title", "LoadGame", false));
    let MenuElementKind::LayoutGroup(group) = &mgr.get_template("title").unwrap().elements[0].kind else {
        panic!("expected layout group");
    };
    assert!(!group.children[1].element.enabled);
    assert!(group.children[0].element.enabled);
}

#[test]
fn set_element_enabled_returns_false_for_missing_menu_id() {
    let mut mgr = MenuManager::new();
    assert!(!mgr.set_element_enabled("nonexistent", "BtnA", false));
}

#[test]
fn set_element_enabled_returns_false_for_missing_element_name() {
    let mut mgr = MenuManager::new();
    mgr.register_template(flat_test_template());
    assert!(!mgr.set_element_enabled("test", "Missing", false));
}

#[test]
fn set_element_visible_toggles_and_returns_true() {
    let mut mgr = MenuManager::new();
    mgr.register_template(flat_test_template());

    assert!(mgr.set_element_visible("test", "BtnB", false));
    let tmpl = mgr.get_template("test").unwrap();
    assert!(!tmpl.elements.iter().find(|e| e.name == "BtnB").unwrap().visible);
}

#[test]
fn open_menu_queues_non_empty_on_open_callback() {
    let mut mgr = MenuManager::new();
    mgr.register_template(grouped_test_template("save_manager.on_title_menu_open"));

    mgr.open_menu("title");

    assert_eq!(
        mgr.take_pending_on_open().as_deref(),
        Some("save_manager.on_title_menu_open")
    );
}

#[test]
fn open_menu_ignores_blank_on_open_callback() {
    let mut mgr = MenuManager::new();
    mgr.register_template(grouped_test_template("   "));

    mgr.open_menu("title");

    assert_eq!(mgr.take_pending_on_open(), None);
}
