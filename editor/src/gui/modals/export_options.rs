use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::storage::export_dispatch::{host_platform, supported_native_target, ExportTarget, NativePlatform};
use bishop::prelude::*;
use engine_core::controls::Controls;
use engine_core::ui::measure_text;
use std::cell::RefCell;
use std::thread::LocalKey;
use widgets::{Button, Rect};

thread_local! {
    static EXPORT_OPTIONS_RESULT: RefCell<Option<ExportOptionsResult>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub enum ExportOptionsResult {
    Selected(ExportTarget),
    Cancelled,
}

pub struct ExportOptionsModal;

crate::register_modal!(ExportOptionsModal);

impl ModalHandler for ExportOptionsModal {
    type Result = ExportOptionsResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &EXPORT_OPTIONS_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 420.0, 310.0);
        let rect = editor.modal.rect;
        open_modal_with_prompt(&mut editor.modal, move |ctx| {
            let x = rect.x + 30.0;
            let width = rect.w - 60.0;
            ctx.draw_text("Export target", x, rect.y + 38.0, 18.0, Color::WHITE);
            if Button::new(Rect::new(x, rect.y + 55.0, width, 36.0), "Web (browser player)").show(ctx) {
                return Some(ExportOptionsResult::Selected(ExportTarget::Web));
            }
            let host = host_platform();
            for (index, (platform, label)) in [
                (NativePlatform::Mac, "macOS app"),
                (NativePlatform::Windows, "Windows executable"),
                (NativePlatform::Linux, "Linux executable"),
            ].into_iter().enumerate() {
                let y = rect.y + 98.0 + 39.0 * index as f32;
                if let Some(target) = supported_native_target(platform, host) {
                    if Button::new(Rect::new(x, y, width, 34.0), label).show(ctx) {
                        return Some(ExportOptionsResult::Selected(target));
                    }
                } else {
                    let label = format!("{label} (unavailable)");
                    let dims = measure_text(ctx, &label, 16.0);
                    ctx.draw_text(&label, x + 8.0, y + 8.0 + dims.offset_y, 16.0,
                        Color::WHITE.with_alpha(0.45));
                }
            }
            if Button::new(Rect::new(x + width - 110.0, rect.y + 241.0, 110.0, 34.0), "Cancel").show(ctx)
                || Controls::escape(ctx)
            {
                return Some(ExportOptionsResult::Cancelled);
            }
            None
        }, &EXPORT_OPTIONS_RESULT);
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        editor.modal.close();
        if let ExportOptionsResult::Selected(target) = result {
            editor.select_export_target(ctx, target);
        }
        None
    }
}
