use super::PropertyModule;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use bishop::prelude::*;
use engine_core::ecs::inspector::collapsible_header::CollapsibleHeader;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use std::marker::PhantomData;

/// Collapsible wrapper for a `PropertyModule<T>`.
pub struct CollapsiblePropertyModule<T, M: PropertyModule<T>> {
    header: CollapsibleHeader,
    title: String,
    inner: M,
    _phantom: PhantomData<T>,
}

impl<T, M: PropertyModule<T>> CollapsiblePropertyModule<T, M> {
    pub fn new(inner: M) -> Self {
        let title = inner.title().to_string();
        let header = CollapsibleHeader::new(&title);
        Self {
            header,
            title,
            inner,
            _phantom: PhantomData,
        }
    }

    /// Returns a reference to the inner module.
    pub fn inner(&self) -> &M {
        &self.inner
    }
}

impl<T, M: PropertyModule<T>> PropertyModule<T> for CollapsiblePropertyModule<T, M> {
    fn visible(&self, target: &T, game_ctx: &GameCtxMut) -> bool {
        self.inner.visible(target, game_ctx)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        target: &mut T,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) {
        self.header.draw(ctx, rect, &self.title, false);

        if self.header.expanded() {
            let body_rect = Rect::new(
                rect.x + 4.0,
                rect.y + CollapsibleHeader::HEADER_HEIGHT + 4.0,
                rect.w - 8.0,
                rect.h - CollapsibleHeader::HEADER_HEIGHT - 8.0,
            );
            self.inner.draw(ctx, body_rect, target, game_ctx, insp_ctx);
        }
    }

    fn take_host_action(&mut self) -> Option<InspectorHostAction> {
        self.inner.take_host_action()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.inner.body_layout()
    }

    fn height(&self) -> f32 {
        if self.header.expanded() {
            CollapsibleHeader::HEADER_HEIGHT + self.inner.body_layout().height()
        } else {
            CollapsibleHeader::HEADER_HEIGHT
        }
    }

    fn title(&self) -> &str {
        &self.title
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeModule;
    impl PropertyModule<()> for FakeModule {
        fn draw(
            &mut self,
            _ctx: &mut WgpuContext,
            _rect: Rect,
            _target: &mut (),
            _game_ctx: &mut GameCtxMut,
            _insp_ctx: &InspectorContext,
        ) {}
        fn body_layout(&self) -> InspectorBodyLayout {
            InspectorBodyLayout::new().rows(1, 4.0)
        }
        fn title(&self) -> &str {
            "FakeModule"
        }
    }

    #[test]
    fn starts_expanded() {
        let module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule);
        assert!(module.header.expanded());
    }

    #[test]
    fn height_when_expanded_includes_header_and_body() {
        let module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule);
        let body = InspectorBodyLayout::new().rows(1, 4.0).height();
        assert_eq!(module.height(), CollapsibleHeader::HEADER_HEIGHT + body);
    }

    #[test]
    fn height_when_collapsed_is_header_only() {
        let mut module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule);
        module.header.set_expanded(false);
        assert_eq!(module.height(), CollapsibleHeader::HEADER_HEIGHT);
    }

    #[test]
    fn body_layout_delegates_to_inner() {
        let module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule);
        // FakeModule uses .rows(1, 4.0) which has non-zero height
        assert!(module.body_layout().height() > 0.0);
    }

    #[test]
    fn title_delegates_to_inner() {
        let module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule);
        assert_eq!(module.title(), "FakeModule");
    }

}
