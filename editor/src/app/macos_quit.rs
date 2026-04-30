use bishop::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Weak;

use objc2::runtime::{AnyObject, Sel};
use objc2::sel;
use objc2_app_kit::NSApplication;
use objc2_foundation::MainThreadMarker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MacOsTerminationAction {
    CancelAndRequestClose,
    AllowTerminate,
}

fn macos_termination_action(exit_confirmed: bool) -> MacOsTerminationAction {
    if exit_confirmed {
        MacOsTerminationAction::AllowTerminate
    } else {
        MacOsTerminationAction::CancelAndRequestClose
    }
}

thread_local! {
    static MACOS_QUIT_TARGET: RefCell<Option<MacOsQuitTarget>> = const { RefCell::new(None) };
}

thread_local! {
    static ORIGINAL_TERMINATE: Cell<Option<MacOsTerminateFn>> = const { Cell::new(None) };
}

type MacOsTerminateFn = extern "C" fn(&NSApplication, Sel, Option<&AnyObject>);

struct MacOsQuitTarget {
    ctx: Weak<RefCell<WgpuContext>>,
}

extern "C" fn macos_terminate(app: &NSApplication, sel: Sel, sender: Option<&AnyObject>) {
    let action = MACOS_QUIT_TARGET.with(|target| {
        let target = target.borrow();
        let target = target.as_ref()?;
        let ctx = target.ctx.upgrade()?;
        let action = macos_termination_action(ctx.borrow().is_exit_confirmed());
        Some((ctx, action))
    });

    match action {
        Some((ctx, MacOsTerminationAction::CancelAndRequestClose)) => {
            let mut ctx = ctx.borrow_mut();
            ctx.set_close_requested(true);
            ctx.window().request_redraw();
        }
        Some((_, MacOsTerminationAction::AllowTerminate)) | None => {
            ORIGINAL_TERMINATE.with(|original| {
                if let Some(original) = original.get() {
                    original(app, sel, sender);
                }
            });
        }
    }
}

pub(crate) fn install(ctx: &PlatformContext) {
    use objc2::runtime::Imp;

    MACOS_QUIT_TARGET.with(|target| {
        *target.borrow_mut() = Some(MacOsQuitTarget {
            ctx: std::rc::Rc::downgrade(ctx),
        });
    });

    let mtm = MainThreadMarker::new()
        .expect("editor macOS quit interception must be installed on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let class = app.class();
    let method = class
        .instance_method(sel!(terminate:))
        .expect("NSApplication must provide terminate:");
    let overridden = unsafe { std::mem::transmute::<MacOsTerminateFn, Imp>(macos_terminate) };

    #[allow(unknown_lints, unpredictable_function_pointer_comparisons)]
    if overridden == method.implementation() {
        return;
    }

    let original = unsafe { method.set_implementation(overridden) };
    let original = unsafe { std::mem::transmute::<Imp, MacOsTerminateFn>(original) };
    ORIGINAL_TERMINATE.with(|slot| slot.set(Some(original)));
}

pub(crate) fn uninstall() {
    use objc2::runtime::Imp;

    ORIGINAL_TERMINATE.with(|slot| {
        let Some(original) = slot.get() else {
            return;
        };

        let mtm = MainThreadMarker::new()
            .expect("editor macOS quit interception must be uninstalled on the main thread");
        let app = NSApplication::sharedApplication(mtm);
        let class = app.class();
        let method = class
            .instance_method(sel!(terminate:))
            .expect("NSApplication must provide terminate:");
        let overridden = unsafe { std::mem::transmute::<MacOsTerminateFn, Imp>(macos_terminate) };

        #[allow(unknown_lints, unpredictable_function_pointer_comparisons)]
        if method.implementation() == overridden {
            let original = unsafe { std::mem::transmute::<MacOsTerminateFn, Imp>(original) };
            unsafe { method.set_implementation(original) };
        }

        slot.set(None);
    });

    MACOS_QUIT_TARGET.with(|target| *target.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::{macos_termination_action, MacOsTerminationAction};

    #[test]
    fn macos_quit_cancels_termination_until_exit_is_confirmed() {
        assert_eq!(
            macos_termination_action(false),
            MacOsTerminationAction::CancelAndRequestClose,
        );
    }

    #[test]
    fn macos_quit_allows_termination_after_exit_is_confirmed() {
        assert_eq!(
            macos_termination_action(true),
            MacOsTerminationAction::AllowTerminate,
        );
    }
}
