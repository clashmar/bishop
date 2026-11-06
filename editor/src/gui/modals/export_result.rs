use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, take_modal_result, Modal, ModalHandler, ModalResult};
use bishop::prelude::*;
use engine_core::ui::Toast;
use std::cell::RefCell;
use std::thread::LocalKey;
use widgets::{Button, Rect};

thread_local! {
    static EXPORT_RESULT: RefCell<Option<ExportResultAction>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub enum ExportResultAction {
    Preview,
    OpenFolder,
    Close,
}

pub struct ExportResultModal;

crate::register_modal!(ExportResultModal);

impl ModalHandler for ExportResultModal {
    type Result = ExportResultAction;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &EXPORT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 420.0, 165.0);
        let rect = editor.modal.rect;
        open_modal_with_prompt(&mut editor.modal, move |ctx| {
            ctx.draw_text("Web export ready", rect.x + 30.0, rect.y + 43.0, 18.0, Color::WHITE);
            let y = rect.y + 93.0;
            if Button::new(Rect::new(rect.x + 30.0, y, 110.0, 36.0), "Preview").show(ctx) {
                return Some(ExportResultAction::Preview);
            }
            if Button::new(Rect::new(rect.x + 150.0, y, 110.0, 36.0), "Open folder").show(ctx) {
                return Some(ExportResultAction::OpenFolder);
            }
            if Button::new(Rect::new(rect.x + 270.0, y, 110.0, 36.0), "Close").show(ctx) {
                return Some(ExportResultAction::Close);
            }
            None
        }, &EXPORT_RESULT);
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            ExportResultAction::Preview => editor.open_web_export_preview(),
            ExportResultAction::OpenFolder => editor.open_web_export_folder(),
            ExportResultAction::Close => {
                editor.web_preview = None;
                editor.last_web_export = None;
                editor.modal.close();
            }
        }
        None
    }

    fn on_outside_click(&mut self, editor: &mut Editor) {
        take_modal_result(&EXPORT_RESULT);
        if editor.last_web_export.take().is_some() {
            editor.web_preview = None;
            editor.modal.close();
            editor.toast = Some(Toast::new("Web preview closed.", 2.5));
        }
    }
}
