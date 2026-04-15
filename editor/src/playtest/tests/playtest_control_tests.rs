use crate::app::Editor;

#[test]
fn open_playtest_for_current_room_rejects_missing_room() {
    let mut editor = Editor::default();

    assert!(editor.launch_playtest_for_current_room().is_err());
}
