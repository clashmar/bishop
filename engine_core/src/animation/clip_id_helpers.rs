use super::clips::ClipId;
use strum::IntoEnumIterator;

pub(crate) fn builtin_clip_ids() -> impl Iterator<Item = ClipId> {
    ClipId::iter().filter(|clip_id| !matches!(clip_id, ClipId::Custom(_) | ClipId::New))
}

pub(crate) fn clip_id_from_name(name: &str) -> ClipId {
    builtin_clip_ids()
        .find(|clip_id| clip_id.canonical_name() == name)
        .unwrap_or_else(|| ClipId::Custom(name.to_string()))
}

pub(crate) fn json_filename(clip_id: &ClipId) -> String {
    format!("{}.json", clip_id.canonical_name())
}

pub(crate) fn sprite_filename(clip_id: &ClipId) -> Option<String> {
    if *clip_id == ClipId::New {
        return None;
    }

    Some(format!("{}.png", clip_id.canonical_name()))
}
