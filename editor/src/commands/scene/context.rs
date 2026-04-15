use crate::app::EditorMode;
use crate::Editor;
use engine_core::prelude::*;

pub(super) fn uses_prefab_context(mode: EditorMode) -> bool {
    matches!(mode, EditorMode::Prefab(_))
}

pub(super) fn with_scene_ctx<T>(
    editor: &mut Editor,
    mode: EditorMode,
    f: impl FnOnce(&mut dyn EngineCtxMut) -> T,
) -> T {
    if uses_prefab_context(mode) {
        let mut prefab_ctx = editor
            .prefab_stage
            .as_mut()
            .expect("Prefab stage missing")
            .ctx_mut();
        f(&mut prefab_ctx)
    } else {
        let mut game_ctx = editor.game.ctx_mut();
        let mut services_ctx = game_ctx.services_ctx_mut();
        f(&mut services_ctx)
    }
}

pub(super) fn with_scene_ecs<T>(
    editor: &mut Editor,
    mode: EditorMode,
    f: impl FnOnce(&mut Ecs) -> T,
) -> T {
    with_scene_ctx(editor, mode, |ctx| f(ctx.ecs()))
}
