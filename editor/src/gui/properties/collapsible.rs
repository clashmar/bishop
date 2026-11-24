use super::PropertyModule;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use bishop::prelude::*;
use engine_core::ecs::inspector::collapsible_header::CollapsibleHeader;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use std::marker::PhantomData;

pub(crate) const COLLAPSIBLE_CONTENT_INSET: f32 = 4.0;

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

    pub(crate) fn inner_mut(&mut self) -> &mut M {
        &mut self.inner
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
                rect.x + COLLAPSIBLE_CONTENT_INSET,
                rect.y + CollapsibleHeader::HEADER_HEIGHT + COLLAPSIBLE_CONTENT_INSET,
                rect.w - COLLAPSIBLE_CONTENT_INSET * 2.0,
                rect.h - CollapsibleHeader::HEADER_HEIGHT - COLLAPSIBLE_CONTENT_INSET * 2.0,
            );
            self.inner.draw(ctx, body_rect, target, game_ctx, insp_ctx);
        }
    }

    fn take_host_action(&mut self) -> Option<InspectorHostAction> {
        self.inner.take_host_action()
    }

    fn expansion_state(&self) -> Option<bool> {
        Some(self.header.expanded())
    }

    fn set_expanded(&mut self, expanded: bool) {
        self.header
            .set_expanded_and_persist(&self.title, expanded);
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

    struct FakeModule(String);
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
            &self.0
        }
    }

    #[test]
    fn property_module_expansion_state_tracks_setter() {
        let title = format!("PropertyExpansionSetterTest-{}", std::process::id());
        let mut module =
            CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule(title));

        assert_eq!(module.expansion_state(), Some(true));
        module.set_expanded(false);
        assert_eq!(module.expansion_state(), Some(false));
        module.set_expanded(true);
    }

    #[test]
    fn inspector_expansion_reveal_rect_requires_collapsed_to_expanded_transition() {
        let mut module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule(
            "PropertyExpansionRevealTest".to_owned(),
        ));
        let rect = Rect::new(10.0, 20.0, 100.0, 24.0);

        assert_eq!(module.expansion_reveal_rect(Some(true), rect), None);
        module.header.set_expanded(false);
        let previous_state = module.expansion_state();
        assert_eq!(module.expansion_reveal_rect(previous_state, rect), None);
        module.header.set_expanded(true);

        assert_eq!(
            module.expansion_reveal_rect(previous_state, rect),
            Some(Rect::new(rect.x, rect.y, rect.w, module.height()))
        );
        assert_eq!(module.expansion_reveal_rect(Some(true), rect), None);
    }

    #[test]
    fn starts_expanded() {
        let module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule(
            "FakeModule".to_owned(),
        ));
        assert!(module.header.expanded());
    }

    #[test]
    fn height_when_expanded_includes_header_and_body() {
        let module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule(
            "FakeModule".to_owned(),
        ));
        let body = InspectorBodyLayout::new().rows(1, 4.0).height();
        assert_eq!(module.height(), CollapsibleHeader::HEADER_HEIGHT + body);
    }

    #[test]
    fn height_when_collapsed_is_header_only() {
        let mut module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule(
            "FakeModule".to_owned(),
        ));
        module.header.set_expanded(false);
        assert_eq!(module.height(), CollapsibleHeader::HEADER_HEIGHT);
    }

    #[test]
    fn body_layout_delegates_to_inner() {
        let module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule(
            "FakeModule".to_owned(),
        ));
        // FakeModule uses .rows(1, 4.0) which has non-zero height
        assert!(module.body_layout().height() > 0.0);
    }

    #[test]
    fn title_delegates_to_inner() {
        let module = CollapsiblePropertyModule::<(), FakeModule>::new(FakeModule(
            "FakeModule".to_owned(),
        ));
        assert_eq!(module.title(), "FakeModule");
    }

}
