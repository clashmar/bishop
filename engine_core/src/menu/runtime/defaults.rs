use crate::menu::*;
use bishop::prelude::*;

/// Builds the engine's built-in runtime menus.
pub(crate) fn default_menus() -> Vec<MenuTemplate> {
    let layout = LayoutConfig::vertical()
        .with_item_size(200.0, 40.0)
        .with_spacing(16.0)
        .with_padding(Padding::uniform(32.0))
        .with_alignment(Alignment::center());

    let pause_menu = MenuBuilder::new("pause")
        .background(MenuBackground::Dimmed(0.7))
        .layout_group(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            layout,
            |group| group.label("Paused").button("Resume", MenuAction::Resume),
        )
        .build();

    let settings_layout = LayoutConfig::vertical()
        .with_item_size(300.0, 40.0)
        .with_spacing(16.0)
        .with_padding(Padding::uniform(32.0))
        .with_alignment(Alignment::center());

    let settings_menu = MenuBuilder::new("settings")
        .background(MenuBackground::Dimmed(0.7))
        .layout_group(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            settings_layout,
            |group| {
                group
                    .label("Settings")
                    .slider("Master Volume", "master_volume", 0.0, 1.0, 0.05, 1.0)
                    .slider("Music Volume", "music_volume", 0.0, 1.0, 0.05, 1.0)
                    .slider("SFX Volume", "sfx_volume", 0.0, 1.0, 0.05, 1.0)
                    .button("Back", MenuAction::CloseMenu)
            },
        )
        .build();

    vec![pause_menu, settings_menu]
}
