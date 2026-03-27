use crate::menu::*;
use bishop::prelude::*;

/// A focusable menu target resolved into screen space.
#[derive(Debug, Clone, PartialEq)]
struct FocusTarget {
    focus: MenuFocus,
    rect: Rect,
}

/// Resolves the focus target under the given mouse position.
pub(crate) fn focus_target_at(
    template: &MenuTemplate,
    viewport: Rect,
    mouse: Vec2,
) -> Option<MenuFocus> {
    let canvas_origin = Vec2::new(viewport.x, viewport.y);
    let canvas_size = Vec2::new(viewport.w, viewport.h);

    collect_focus_targets(template, canvas_origin, canvas_size)
        .into_iter()
        .rev()
        .find(|target| target.rect.contains(mouse))
        .map(|target| target.focus)
}

fn collect_focus_targets(
    template: &MenuTemplate,
    canvas_origin: Vec2,
    canvas_size: Vec2,
) -> Vec<FocusTarget> {
    let mut targets = Vec::new();

    for element_index in template.sorted_element_indices() {
        let element = &template.elements[element_index];
        if !element.visible {
            continue;
        }

        match &element.kind {
            MenuElementKind::Button(_) | MenuElementKind::Slider(_) if element.enabled => {
                targets.push(FocusTarget {
                    focus: MenuFocus::new(element_index),
                    rect: normalized_rect_to_screen(element.rect, canvas_origin, canvas_size),
                });
            }
            MenuElementKind::LayoutGroup(group) => {
                let resolved = resolve_layout(group, element.rect);
                let mut child_focus_index = 0;

                for (child, rect) in group.children.iter().zip(resolved.iter()) {
                    if !child.element.visible {
                        continue;
                    }

                    let is_focusable = matches!(
                        child.element.kind,
                        MenuElementKind::Button(_) | MenuElementKind::Slider(_)
                    );

                    if is_focusable && child.element.enabled {
                        targets.push(FocusTarget {
                            focus: MenuFocus {
                                node: element_index,
                                child: Some(child_focus_index),
                            },
                            rect: normalized_rect_to_screen(*rect, canvas_origin, canvas_size),
                        });
                        child_focus_index += 1;
                    }
                }
            }
            _ => {}
        }
    }

    targets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_focus_targets_includes_top_level_and_layout_children() {
        let mut template = MenuTemplate::new("settings".to_string());
        template.elements.push(MenuElement::label(
            "title".to_string(),
            Rect::new(0.0, 0.0, 1.0, 0.1),
        ));
        template.elements.push(MenuElement::button(
            "resume".to_string(),
            MenuAction::Resume,
            Rect::new(0.1, 0.2, 0.2, 0.1),
        ));
        template.elements.push(MenuElement::layout_group(
            LayoutGroupElement {
                layout: LayoutConfig::vertical()
                    .with_item_size(200.0, 40.0)
                    .with_spacing(10.0),
                children: vec![
                    LayoutChild {
                        element: MenuElement::label(
                            "header".to_string(),
                            Rect::new(0.0, 0.0, 0.0, 0.0),
                        ),
                        managed: true,
                    },
                    LayoutChild {
                        element: MenuElement::button(
                            "apply".to_string(),
                            MenuAction::CloseMenu,
                            Rect::new(0.0, 0.0, 0.0, 0.0),
                        ),
                        managed: true,
                    },
                    LayoutChild {
                        element: MenuElement::slider(
                            "volume".to_string(),
                            "master".to_string(),
                            0.0,
                            1.0,
                            0.1,
                            0.5,
                            Rect::new(0.0, 0.0, 0.0, 0.0),
                        ),
                        managed: true,
                    },
                ],
                ..Default::default()
            },
            Rect::new(0.0, 0.3, 1.0, 0.6),
        ));

        let targets = collect_focus_targets(&template, Vec2::ZERO, Vec2::new(1000.0, 500.0));

        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].focus, MenuFocus::new(1));
        assert_eq!(
            targets[1].focus,
            MenuFocus {
                node: 2,
                child: Some(0),
            }
        );
        assert_eq!(
            targets[2].focus,
            MenuFocus {
                node: 2,
                child: Some(1),
            }
        );
    }

    #[test]
    fn focus_target_at_returns_topmost_matching_focus() {
        let mut back = MenuElement::button(
            "back".to_string(),
            MenuAction::CloseMenu,
            Rect::new(0.1, 0.1, 0.4, 0.2),
        );
        back.z_order = 0;

        let mut front = MenuElement::button(
            "front".to_string(),
            MenuAction::Resume,
            Rect::new(0.1, 0.1, 0.4, 0.2),
        );
        front.z_order = 1;

        let template = MenuTemplate {
            id: "overlay".to_string(),
            background: MenuBackground::None,
            elements: vec![back, front],
            mode: MenuMode::Paused,
        };

        let focus = focus_target_at(
            &template,
            Rect::new(0.0, 0.0, 1000.0, 500.0),
            Vec2::new(200.0, 100.0),
        );

        assert_eq!(focus, Some(MenuFocus::new(1)));
    }
}
