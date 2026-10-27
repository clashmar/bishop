use crate::types::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClipRegion {
    Unclipped,
    Empty,
    Rect(Rect),
}

#[derive(Clone, Default)]
pub(crate) struct ClipStack {
    stack: Vec<Option<Rect>>,
}

impl ClipStack {
    pub(crate) fn push(&mut self, rect: Rect) {
        let active = match self.current() {
            ClipRegion::Unclipped => Some(rect),
            ClipRegion::Empty => None,
            ClipRegion::Rect(parent) => parent.intersection(&rect),
        };
        self.stack.push(active);
    }

    pub(crate) fn pop(&mut self) {
        self.stack.pop();
    }

    pub(crate) fn current(&self) -> ClipRegion {
        match self.stack.last() {
            None => ClipRegion::Unclipped,
            Some(None) => ClipRegion::Empty,
            Some(Some(rect)) => ClipRegion::Rect(*rect),
        }
    }
}

pub(crate) fn scissor_y(framebuffer_height: i32, y: i32, height: i32) -> i32 {
    framebuffer_height - (y + height)
}

#[cfg(test)]
mod tests {
    use super::{ClipRegion, ClipStack, scissor_y};
    use crate::types::Rect;

    #[test]
    fn nested_clip_intersects_and_pop_restores_parent() {
        let parent = Rect::new(8.0, 10.0, 40.0, 48.0);
        let child = Rect::new(20.0, 18.0, 40.0, 12.0);
        let mut clips = ClipStack::default();

        clips.push(parent);
        clips.push(child);
        assert_eq!(
            clips.current(),
            ClipRegion::Rect(Rect::new(20.0, 18.0, 28.0, 12.0))
        );

        clips.pop();
        assert_eq!(clips.current(), ClipRegion::Rect(parent));
        clips.pop();
        assert_eq!(clips.current(), ClipRegion::Unclipped);
    }

    #[test]
    fn empty_intersection_remains_clipped_until_popped() {
        let mut clips = ClipStack::default();
        clips.push(Rect::new(0.0, 0.0, 10.0, 10.0));
        clips.push(Rect::new(20.0, 20.0, 4.0, 4.0));

        assert_eq!(clips.current(), ClipRegion::Empty);

        clips.pop();
        assert_eq!(
            clips.current(),
            ClipRegion::Rect(Rect::new(0.0, 0.0, 10.0, 10.0))
        );
    }

    #[test]
    fn top_first_clip_converts_to_bottom_first_scissor_y() {
        assert_eq!(scissor_y(90, 18, 12), 60);
    }
}
