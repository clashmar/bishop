use crate::editor_global::push_toast;
use crate::gui::gui_constants::INPUT_HEIGHT;
use bishop::prelude::*;
use engine_core::animation::{ClipDef, ClipId};
use engine_core::ecs::{Animation, Ecs, Entity};
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;
use widgets::constants::layout;
use widgets::{text_input_reset, Dropdown, InputCommit, TextInput, Widget};

use super::frame_edit::sync_selected_clip;
use super::AnimationModule;

pub(super) fn draw_current_clip_dropdowns(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    rect: Rect,
    entity: Entity,
    animation: &mut Animation,
    all_ids: Vec<ClipId>,
    blocked: bool,
) -> bool {
    let Some(current_id) = animation.current.as_ref() else {
        return false;
    };
    let clip_label = format!("{current_id}");
    let width = rect.w / 2.0 - layout::WIDGET_SPACING;

    // Select clip
    let select_rect = Rect::new(rect.x, rect.y, width, rect.h);

    if let Some(selected) = Dropdown::new(
        module.select_dropdown_id,
        select_rect,
        &clip_label,
        &existing_clip_ids(&animation.clips),
        |id| id.canonical_name(),
    )
    .suppressed(blocked)
    .show(ctx)
    {
        animation.set_clip(&selected);
        sync_selected_clip(entity, animation, selected);
        return false;
    }

    // Edit the ClipId of the current clip
    let right_rect = Rect::new((select_rect.x + rect.w) - (width), rect.y, width, rect.h);

    // Show the type selector
    let type_label = "Set Type";

    let chosen = Dropdown::new(
        module.set_dropdown_id,
        right_rect,
        type_label,
        &all_ids,
        |id| id.canonical_name(),
    )
    .suppressed(blocked)
    .show(ctx);

    if let Some(chosen) = chosen {
        match chosen {
            // For now always open the rename field
            ClipId::New => {
                module.pending_rename = true;
                module.rename_initial_value.clear();
                return false;
            }
            ClipId::Custom(name) => {
                module.pending_rename = true;
                module.rename_initial_value = name.clone();
                return false;
            }
            // Any other enum variant
            other => {
                // Prevent duplicate concrete types on the same entity
                if animation.clips.contains_key(&other)
                    && Some(&other) != animation.current.as_ref()
                {
                    push_toast("Entity already has this animation.", 2.0);
                } else {
                    reset_current_clip_id(animation, other.clone());
                    sync_selected_clip(entity, animation, other);
                    module.pending_rename = false;
                    return true;
                }
                return false;
            }
        }
    }

    // Render the rename text field while the flag is true
    if module.pending_rename {
        // Position directly under the right‑hand dropdown
        let input_rect = Rect::new(
            right_rect.x,
            right_rect.y + right_rect.h + 4.0,
            right_rect.w,
            INPUT_HEIGHT,
        );

        const CLAMP: usize = 12;

        // The field starts empty each time we open it
        let (entered, commit) = TextInput::new(
            module.rename_field_id,
            input_rect,
            &module.rename_initial_value,
        )
        .max_len(CLAMP)
        .focused(true)
        .blocked(blocked)
        .show(ctx);

        // Check if enter is pressed first
        if ctx.is_key_pressed(KeyCode::Enter) {
            let new_id = ClipId::Custom(entered.trim().to_string());
            reset_current_clip_id(animation, new_id.clone());
            sync_selected_clip(entity, animation, new_id);
            module.pending_rename = false;
            text_input_reset(module.rename_field_id);
            return true;
        } else if !matches!(commit, InputCommit::Previewing) {
            text_input_reset(module.rename_field_id);
            module.pending_rename = false;
        }
    }

    false
}

/// Adds every possible `ClipId` to the supplied Vec.
pub(super) fn fill_all_clip_ids(ecs: &Ecs, out: &mut Vec<ClipId>) {
    // Built‑in IDs
    let mut ids: Vec<ClipId> = ClipId::iter()
        .filter(|id| !matches!(id, ClipId::New | ClipId::Custom(_)))
        .collect();

    // Gather every custom type
    let mut custom_names = HashSet::new();
    for animation in ecs.get_store::<Animation>().data.values() {
        for clip_id in animation.clips.keys() {
            if let ClipId::Custom(name) = clip_id {
                custom_names.insert(name.clone());
            }
        }
    }

    // Sort the custom values
    let mut custom_ids: Vec<ClipId> = custom_names.into_iter().map(ClipId::Custom).collect();

    custom_ids.sort_by_key(|id| id.canonical_name());

    // Assemble the final list with New at the end
    ids.extend(custom_ids);
    ids.push(ClipId::New);
    *out = ids;
}

/// Returns every ClipId that has a concrete Clip stored in the map.
fn existing_clip_ids(clips: &HashMap<ClipId, ClipDef>) -> Vec<ClipId> {
    clips.keys().cloned().collect()
}

/// Helper that moves the currently selected clip under a new `ClipId`.
fn reset_current_clip_id(animation: &mut Animation, new_id: ClipId) {
    let Some(old_id) = animation.current.take() else {
        return;
    };

    // Take the old clip out of the map
    if let Some(old_clip) = animation.clips.remove(&old_id) {
        // Insert it under the new key
        animation.clips.insert(new_id.clone(), old_clip);
    }

    // Move the runtime state as well
    if let Some(state) = animation.states.remove(&old_id) {
        animation.states.insert(new_id.clone(), state);
    }

    // Finally make the renamed clip the active one
    animation.current = Some(new_id);
}
