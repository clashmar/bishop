use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use bishop::text::FontId;

/// Undoable change to the game's default text font.
#[derive(Debug)]
pub struct SetGameDefaultFontCmd {
    new_font: Option<FontId>,
    old_font: Option<FontId>,
}

impl SetGameDefaultFontCmd {
    /// Creates a game default font change.
    pub fn new(new_font: Option<FontId>, old_font: Option<FontId>) -> Self {
        Self { new_font, old_font }
    }
}

impl EditorCommand for SetGameDefaultFontCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            editor.game.text_settings.default_font = self.new_font;
            editor.save();
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            editor.game.text_settings.default_font = self.old_font;
            editor.save();
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Game(_))
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{Editor, EditorMode};
    use crate::commands::editor_command_manager::EditorCommand;
    use crate::editor_global::with_editor;
    use crate::storage::game_io::create_new_game;
    use crate::test_utils::{game_fs_test_lock, EditorServicesGuard, TestGameFolder};
    use engine_core::engine_global::set_game_name;
    use engine_core::text::GameTextSettings;
    use bishop::text::FontId;

    use super::SetGameDefaultFontCmd;

    #[test]
    fn setting_game_default_font_updates_and_undo_restores_previous_value() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let folder = TestGameFolder::new("set_game_default_font_cmd");
        set_game_name(folder.name());
        let editor = Editor {
            game: create_new_game(folder.name().to_string()),
            mode: EditorMode::Game(crate::game::GameEditorSubmode::Worlds),
            ..Default::default()
        };
        let _guard = EditorServicesGuard::install(editor);
        let original = GameTextSettings::default().default_font;
        let selected = Some(FontId(13));
        let mut command = SetGameDefaultFontCmd::new(selected, original);

        command.execute();
        with_editor(|editor| assert_eq!(editor.game.text_settings.default_font, selected));

        command.undo();
        with_editor(|editor| assert_eq!(editor.game.text_settings.default_font, original));
    }
}
