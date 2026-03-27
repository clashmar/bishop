// engine_core/src/ecs/reflect.rs
use crate::assets::sprite::SpriteId;
use crate::ecs::transform::Pivot;
use bishop::types::{Vec2, Vec3};
use std::borrow::Cow;

/// One mutable field value.
pub enum FieldValue<'a> {
    Text(&'a mut String),
    Float(&'a mut f32),
    Int(&'a mut i32),
    Bool(&'a mut bool),
    Vec2(&'a mut Vec2),
    Vec3(&'a mut Vec3),
    SpriteId(&'a mut SpriteId),
    Pivot(&'a mut Pivot),
}

/// Metadata that the inspector consumes.
pub struct FieldInfo<'a> {
    pub name: &'static str,
    pub value: FieldValue<'a>,
    pub widget_hint: Option<&'static str>,
}

/// Trait that every component needs to expose.
pub trait Reflect {
    /// Returns a vector of mutable descriptors for all fields.
    fn fields(&mut self) -> Vec<FieldInfo<'_>>;
}

/// Helper trait
pub trait ReflectField {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a>;
}

/// Helper that attaches a hint to a `FieldInfo`.
impl<'a> FieldInfo<'a> {
    /// Consumes `self` and returns a new `FieldInfo` with the supplied hint.
    pub fn with_hint(mut self, hint: Option<&'static str>) -> Self {
        self.widget_hint = hint;
        self
    }
}

impl ReflectField for String {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a> {
        FieldInfo {
            name,
            value: FieldValue::Text(field),
            widget_hint: None,
        }
    }
}

impl ReflectField for f32 {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a> {
        FieldInfo {
            name,
            value: FieldValue::Float(field),
            widget_hint: None,
        }
    }
}

impl ReflectField for i32 {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a> {
        FieldInfo {
            name,
            value: FieldValue::Int(field),
            widget_hint: None,
        }
    }
}

impl ReflectField for bool {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a> {
        FieldInfo {
            name,
            value: FieldValue::Bool(field),
            widget_hint: None,
        }
    }
}

impl ReflectField for Vec2 {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a> {
        FieldInfo {
            name,
            value: FieldValue::Vec2(field),
            widget_hint: None,
        }
    }
}

impl ReflectField for Vec3 {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a> {
        FieldInfo {
            name,
            value: FieldValue::Vec3(field),
            widget_hint: None,
        }
    }
}

impl ReflectField for SpriteId {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a> {
        FieldInfo {
            name,
            value: FieldValue::SpriteId(field),
            widget_hint: None,
        }
    }
}

impl ReflectField for Pivot {
    fn field_info<'a>(field: &'a mut Self, name: &'static str) -> FieldInfo<'a> {
        FieldInfo {
            name,
            value: FieldValue::Pivot(field),
            widget_hint: None,
        }
    }
}

pub fn parse_field_name(name: &str) -> Cow<'_, str> {
    // Fast path
    if !name.contains('_')
        && name
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
    {
        return Cow::Borrowed(name);
    }

    // Split on '_' and capitalise each segment
    let mut parts = name.split('_').filter(|s| !s.is_empty());

    // Build the first part (to avoid an extra allocation when possible)
    let first = match parts.next() {
        Some(p) => {
            let mut chars = p.chars();
            let first_char = chars.next().map(|c| c.to_ascii_uppercase());
            let rest: String = chars.collect();
            match first_char {
                Some(f) => format!("{}{}", f, rest),
                None => String::new(),
            }
        }
        None => return Cow::Borrowed(name), // empty input
    };

    // Append the remaining parts, each preceded by a space
    let result = parts.fold(first, |mut acc, part| {
        let mut chars = part.chars();
        let first_char = chars.next().map(|c| c.to_ascii_uppercase());
        let rest: String = chars.collect();
        if let Some(f) = first_char {
            acc.push(' ');
            acc.push_str(&format!("{}{}", f, rest));
        }
        acc
    });

    Cow::Owned(result)
}
