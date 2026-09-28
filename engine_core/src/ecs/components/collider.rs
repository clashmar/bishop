use crate::animation::ClipId;
use bishop::prelude::Vec2;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::BTreeMap;
use strum_macros::EnumIter;

/// Default width and height for colliders without a sprite or animation reference.
pub const DEFAULT_COLLIDER_DIMENSION: f32 = 16.0;

#[ecs_component]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Collider {
    pub shape: ColliderShape,
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub offset: Vec2,
    pub animation: ColliderAnimation,
}

/// Shape and offset data for one collider target.
#[serde_as]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ColliderData {
    pub shape: ColliderShape,
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub offset: Vec2,
}

/// Frame coordinates inside one animation clip.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColliderFrameKey {
    pub row: usize,
    pub col: usize,
}

/// Sparse animation collider data keyed by clip and frame.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ColliderAnimation {
    pub clips: BTreeMap<ClipId, ColliderClipData>,
}

/// Sparse collider data for one animation clip.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ColliderClipData {
    pub clip: Option<ColliderData>,
    pub frames: BTreeMap<ColliderFrameKey, ColliderData>,
}

/// Editable collider target selected by editor tooling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColliderEditTarget {
    Static,
    Clip(ClipId),
    Frame {
        clip_id: ClipId,
        frame: ColliderFrameKey,
    },
}

impl ColliderData {
    /// Returns this data as a standalone collider.
    pub fn to_collider(self) -> Collider {
        Collider {
            shape: self.shape,
            offset: self.offset,
            animation: ColliderAnimation::default(),
        }
    }
}

impl Collider {
    /// Returns the static collider data.
    pub fn static_data(&self) -> ColliderData {
        ColliderData {
            shape: self.shape,
            offset: self.offset,
        }
    }

    /// Replaces the static collider data.
    pub fn set_static_data(&mut self, data: ColliderData) {
        self.shape = data.shape;
        self.offset = data.offset;
    }

    /// Returns the effective data for a clip and frame.
    pub fn effective_data_for_frame(
        &self,
        clip_id: &ClipId,
        frame: ColliderFrameKey,
    ) -> ColliderData {
        self.animation
            .clips
            .get(clip_id)
            .and_then(|clip| clip.frames.get(&frame).copied().or(clip.clip))
            .unwrap_or_else(|| self.static_data())
    }

    /// Returns the effective data for an edit target.
    pub fn effective_data_for_target(&self, target: &ColliderEditTarget) -> ColliderData {
        match target {
            ColliderEditTarget::Static => self.static_data(),
            ColliderEditTarget::Clip(clip_id) => self
                .animation
                .clips
                .get(clip_id)
                .and_then(|clip| clip.clip)
                .unwrap_or_else(|| self.static_data()),
            ColliderEditTarget::Frame { clip_id, frame } => {
                self.effective_data_for_frame(clip_id, *frame)
            }
        }
    }

    /// Writes clip-level collider data.
    pub fn set_clip_data(&mut self, clip_id: ClipId, data: ColliderData) {
        self.animation.clips.entry(clip_id).or_default().clip = Some(data);
    }

    /// Writes frame-level collider data.
    pub fn set_frame_data(
        &mut self,
        clip_id: ClipId,
        frame: ColliderFrameKey,
        data: ColliderData,
    ) {
        self.animation
            .clips
            .entry(clip_id)
            .or_default()
            .frames
            .insert(frame, data);
    }

    /// Removes frame-level collider data.
    pub fn clear_frame_data(&mut self, clip_id: &ClipId, frame: ColliderFrameKey) {
        if let Some(clip) = self.animation.clips.get_mut(clip_id) {
            clip.frames.remove(&frame);
            if clip.clip.is_none() && clip.frames.is_empty() {
                self.animation.clips.remove(clip_id);
            }
        }
    }

    /// Mutates an edit target, creating sparse data from fallback when needed.
    pub fn mutate_target(
        &mut self,
        target: ColliderEditTarget,
        mutate: impl FnOnce(&mut ColliderData),
    ) {
        match target {
            ColliderEditTarget::Static => {
                let mut data = self.static_data();
                mutate(&mut data);
                self.set_static_data(data);
            }
            ColliderEditTarget::Clip(clip_id) => {
                let fallback = self
                    .animation
                    .clips
                    .get(&clip_id)
                    .and_then(|clip| clip.clip)
                    .unwrap_or_else(|| self.static_data());
                let slot = &mut self.animation.clips.entry(clip_id).or_default().clip;
                let data = slot.get_or_insert(fallback);
                mutate(data);
            }
            ColliderEditTarget::Frame { clip_id, frame } => {
                let fallback = self.effective_data_for_frame(&clip_id, frame);
                let data = self
                    .animation
                    .clips
                    .entry(clip_id)
                    .or_default()
                    .frames
                    .entry(frame)
                    .or_insert(fallback);
                mutate(data);
            }
        }
    }
}

/// Shape of a collider for physics and overlap detection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, EnumIter)]
pub enum ColliderShape {
    /// Axis-aligned bounding box.
    Aabb {
        width: f32,
        height: f32,
    },
    /// Circle centered on the collider position.
    Circle {
        radius: f32,
    },
    /// Vertical capsule: two half-circles connected by a rectangle.
    Capsule {
        radius: f32,
        height: f32,
    },
    /// Single point (zero-area collider).
    Point,
}

impl ColliderShape {
    /// Returns the editor label for this shape.
    pub fn ui_label(&self) -> &'static str {
        match self {
            Self::Aabb { .. } => "AABB",
            Self::Circle { .. } => "Circle",
            Self::Capsule { .. } => "Capsule",
            Self::Point => "Point",
        }
    }

    /// Returns this shape converted to the selected variant.
    pub fn convert_to(self, selected: Self) -> Self {
        match selected {
            Self::Aabb { .. } => match self {
                Self::Aabb { .. } => self,
                Self::Circle { radius } => Self::Aabb {
                    width: radius * 2.0,
                    height: radius * 2.0,
                },
                Self::Capsule { radius, height } => Self::Aabb {
                    width: radius * 2.0,
                    height: height + radius * 2.0,
                },
                Self::Point => Self::default(),
            },
            Self::Circle { .. } => match self {
                Self::Aabb { width, height } => Self::Circle {
                    radius: width.min(height) / 2.0,
                },
                Self::Circle { .. } => self,
                Self::Capsule { radius, .. } => Self::Circle { radius },
                Self::Point => Self::Circle { radius: DEFAULT_COLLIDER_DIMENSION / 2.0 },
            },
            Self::Capsule { .. } => match self {
                Self::Aabb { width, height } => {
                    let radius = width.min(height) / 2.0;
                    Self::Capsule {
                        radius,
                        height: height - radius * 2.0,
                    }
                }
                Self::Circle { radius } => Self::Capsule {
                    radius,
                    height: radius * 2.0,
                },
                Self::Capsule { .. } => self,
                Self::Point => Self::Capsule {
                    radius: DEFAULT_COLLIDER_DIMENSION / 4.0,
                    height: DEFAULT_COLLIDER_DIMENSION,
                },
            },
            Self::Point => Self::Point,
        }
    }

    /// Returns true if this shape has zero dimensions.
    pub fn is_default_size(&self) -> bool {
        match self {
            Self::Aabb { width, height } => *width == 0.0 && *height == 0.0,
            Self::Circle { radius } => *radius == 0.0,
            Self::Capsule { radius, height } => *radius == 0.0 && *height == 0.0,
            Self::Point => false,
        }
    }

    /// Returns the bounding-box size of this shape.
    pub fn size(&self) -> (f32, f32) {
        match self {
            Self::Aabb { width, height } => (*width, *height),
            Self::Circle { radius } => {
                let diameter = radius * 2.0;
                (diameter, diameter)
            }
            Self::Capsule { radius, height } => (radius * 2.0, height + radius * 2.0),
            Self::Point => (0.0, 0.0),
        }
    }

}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Aabb {
            width: DEFAULT_COLLIDER_DIMENSION,
            height: DEFAULT_COLLIDER_DIMENSION,
        }
    }
}

impl std::fmt::Display for ColliderShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.ui_label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collider_shape_serialization_roundtrip() {
        let shapes = vec![
            ColliderShape::Aabb {
                width: 8.0,
                height: 12.0,
            },
            ColliderShape::Circle { radius: 5.0 },
            ColliderShape::Capsule {
                radius: 3.0,
                height: 10.0,
            },
            ColliderShape::Point,
        ];

        for shape in shapes {
            let ron = ron::ser::to_string(&shape).unwrap();
            let deserialized: ColliderShape = ron::de::from_str(&ron).unwrap();
            assert_eq!(deserialized, shape);
        }
    }

    #[test]
    fn collider_default_is_aabb_with_default_dimension() {
        let collider = Collider::default();
        match collider.shape {
            ColliderShape::Aabb { width, height } => {
                assert_eq!(width, DEFAULT_COLLIDER_DIMENSION);
                assert_eq!(height, DEFAULT_COLLIDER_DIMENSION);
            }
            _ => panic!("default Collider should be Aabb"),
        }
    }

    #[test]
    fn collider_shape_convert_to_aabb_from_circle_uses_diameter() {
        let shape = ColliderShape::Circle { radius: 5.0 };
        assert_eq!(
            shape.convert_to(ColliderShape::Aabb {
                width: 0.0,
                height: 0.0,
            }),
            ColliderShape::Aabb {
                width: 10.0,
                height: 10.0,
            },
        );
    }

    #[test]
    fn collider_shape_convert_to_circle_from_aabb_uses_min_dimension() {
        let shape = ColliderShape::Aabb {
            width: 10.0,
            height: 20.0,
        };
        assert_eq!(
            shape.convert_to(ColliderShape::Circle { radius: 0.0 }),
            ColliderShape::Circle { radius: 5.0 },
        );
    }

    #[test]
    fn collider_shape_convert_to_capsule_from_aabb_preserves_approximate_size() {
        let shape = ColliderShape::Aabb {
            width: 10.0,
            height: 24.0,
        };
        assert_eq!(
            shape.convert_to(ColliderShape::Capsule {
                radius: 0.0,
                height: 0.0,
            }),
            ColliderShape::Capsule {
                radius: 5.0,
                height: 14.0,
            },
        );
    }

    #[test]
    fn collider_shape_convert_to_point_discards_dimensions() {
        let shape = ColliderShape::Aabb {
            width: 10.0,
            height: 20.0,
        };
        assert_eq!(shape.convert_to(ColliderShape::Point), ColliderShape::Point);
    }

    #[test]
    fn collider_shape_convert_to_circle_from_point_uses_default() {
        assert_eq!(
            ColliderShape::Point.convert_to(ColliderShape::Circle { radius: 0.0 }),
            ColliderShape::Circle { radius: DEFAULT_COLLIDER_DIMENSION / 2.0 },
        );
    }

    #[test]
    fn collider_shape_convert_to_same_variant_is_noop() {
        let shape = ColliderShape::Aabb {
            width: 10.0,
            height: 20.0,
        };
        assert_eq!(
            shape.convert_to(ColliderShape::Aabb {
                width: 0.0,
                height: 0.0,
            }),
            shape,
        );
    }

    #[test]
    fn collider_shape_convert_to_aabb_from_capsule_expands_to_bounding_box() {
        let shape = ColliderShape::Capsule {
            radius: 5.0,
            height: 14.0,
        };
        assert_eq!(
            shape.convert_to(ColliderShape::Aabb {
                width: 0.0,
                height: 0.0,
            }),
            ColliderShape::Aabb {
                width: 10.0,
                height: 24.0,
            },
        );
    }

    #[test]
    fn collider_shape_convert_to_capsule_from_circle_doubles_radius_for_height() {
        let shape = ColliderShape::Circle { radius: 5.0 };
        assert_eq!(
            shape.convert_to(ColliderShape::Capsule {
                radius: 0.0,
                height: 0.0,
            }),
            ColliderShape::Capsule {
                radius: 5.0,
                height: 10.0,
            },
        );
    }

    #[test]
    fn collider_shape_convert_to_aabb_from_point_uses_default() {
        assert_eq!(
            ColliderShape::Point.convert_to(ColliderShape::Aabb {
                width: 0.0,
                height: 0.0,
            }),
            ColliderShape::default(),
        );
    }

    #[test]
    fn collider_shape_convert_to_capsule_from_point_uses_default() {
        assert_eq!(
            ColliderShape::Point.convert_to(ColliderShape::Capsule {
                radius: 0.0,
                height: 0.0,
            }),
            ColliderShape::Capsule {
                radius: DEFAULT_COLLIDER_DIMENSION / 4.0,
                height: DEFAULT_COLLIDER_DIMENSION,
            },
        );
    }

    #[test]
    fn collider_shape_is_default_size_zero_dimensions() {
        assert!(ColliderShape::Aabb {
            width: 0.0,
            height: 0.0,
        }
        .is_default_size());
        assert!(ColliderShape::Circle { radius: 0.0 }.is_default_size());
        assert!(ColliderShape::Capsule {
            radius: 0.0,
            height: 0.0,
        }
        .is_default_size());
    }

    #[test]
    fn collider_shape_is_default_size_nonzero_dimensions() {
        assert!(!ColliderShape::Aabb {
            width: 8.0,
            height: 8.0,
        }
        .is_default_size());
        assert!(!ColliderShape::Circle { radius: 5.0 }.is_default_size());
        assert!(!ColliderShape::Capsule {
            radius: 3.0,
            height: 10.0,
        }
        .is_default_size());
    }

    #[test]
    fn point_shape_is_never_default_size() {
        assert!(!ColliderShape::Point.is_default_size());
    }

    #[test]
    fn collider_shape_size_returns_bounding_box() {
        assert_eq!(
            ColliderShape::Aabb {
                width: 8.0,
                height: 12.0,
            }
            .size(),
            (8.0, 12.0),
        );
        assert_eq!(ColliderShape::Circle { radius: 5.0 }.size(), (10.0, 10.0));
        assert_eq!(
            ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            }
            .size(),
            (8.0, 18.0),
        );
        assert_eq!(ColliderShape::Point.size(), (0.0, 0.0));
    }

    #[test]
    fn collider_offset_defaults_to_zero() {
        let collider = Collider::default();
        assert_eq!(collider.offset.x, 0.0);
        assert_eq!(collider.offset.y, 0.0);
    }

    #[test]
    fn collider_offset_serialization_roundtrip() {
        let collider = Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 12.0,
            },
            offset: Vec2::new(3.0, -4.0),
            ..Default::default()
        };
        let ron = ron::ser::to_string(&collider).unwrap();
        let deserialized: Collider = ron::de::from_str(&ron).unwrap();
        assert_eq!(deserialized.offset.x, 3.0);
        assert_eq!(deserialized.offset.y, -4.0);
    }

    #[test]
    fn sparse_collider_lookup_prefers_frame_then_clip_then_static() {
        use crate::animation::ClipId;

        let mut collider = Collider {
            shape: ColliderShape::Aabb {
                width: 10.0,
                height: 20.0,
            },
            offset: Vec2::new(1.0, 2.0),
            ..Default::default()
        };
        let clip_data = ColliderData {
            shape: ColliderShape::Aabb {
                width: 30.0,
                height: 40.0,
            },
            offset: Vec2::new(3.0, 4.0),
        };
        let frame_data = ColliderData {
            shape: ColliderShape::Aabb {
                width: 50.0,
                height: 60.0,
            },
            offset: Vec2::new(5.0, 6.0),
        };
        let frame = ColliderFrameKey { row: 0, col: 2 };

        collider.set_clip_data(ClipId::Run, clip_data);
        collider.set_frame_data(ClipId::Run, frame, frame_data);

        assert_eq!(
            collider.effective_data_for_frame(&ClipId::Run, frame),
            frame_data,
        );
        assert_eq!(
            collider.effective_data_for_frame(&ClipId::Run, ColliderFrameKey { row: 0, col: 1 }),
            clip_data,
        );
        assert_eq!(
            collider.effective_data_for_frame(&ClipId::Idle, frame),
            collider.static_data(),
        );
    }

    #[test]
    fn sparse_collider_mutating_missing_frame_copies_effective_fallback() {
        use crate::animation::ClipId;

        let mut collider = Collider {
            shape: ColliderShape::Aabb {
                width: 10.0,
                height: 20.0,
            },
            offset: Vec2::new(1.0, 2.0),
            ..Default::default()
        };
        let clip_data = ColliderData {
            shape: ColliderShape::Aabb {
                width: 30.0,
                height: 40.0,
            },
            offset: Vec2::new(3.0, 4.0),
        };
        let frame = ColliderFrameKey { row: 1, col: 0 };

        collider.set_clip_data(ClipId::Run, clip_data);
        collider.mutate_target(
            ColliderEditTarget::Frame {
                clip_id: ClipId::Run,
                frame,
            },
            |data| data.offset.x = 99.0,
        );

        let edited = collider.effective_data_for_frame(&ClipId::Run, frame);
        assert_eq!(edited.shape, clip_data.shape);
        assert_eq!(edited.offset, Vec2::new(99.0, 4.0));
        assert_eq!(
            collider.effective_data_for_frame(&ClipId::Run, ColliderFrameKey { row: 1, col: 1 }),
            clip_data,
        );
    }

    #[test]
    fn sparse_collider_serialization_roundtrips_animation_data() {
        use crate::animation::ClipId;

        let mut collider = Collider::default();
        let frame = ColliderFrameKey { row: 0, col: 1 };
        let frame_data = ColliderData {
            shape: ColliderShape::Circle { radius: 7.0 },
            offset: Vec2::new(8.0, 9.0),
        };

        collider.set_frame_data(ClipId::Jump, frame, frame_data);

        let ron = ron::ser::to_string(&collider).unwrap();
        let deserialized: Collider = ron::de::from_str(&ron).unwrap();

        assert_eq!(
            deserialized.effective_data_for_frame(&ClipId::Jump, frame),
            frame_data,
        );
        assert_eq!(
            deserialized.effective_data_for_frame(
                &ClipId::Jump,
                ColliderFrameKey { row: 0, col: 0 },
            ),
            deserialized.static_data(),
        );
    }

    #[test]
    fn collider_offset_deserialization_missing_field() {
        let ron = r#"Collider(shape: Aabb(width: 8.0, height: 12.0))"#;
        let deserialized: Collider = ron::de::from_str(ron).unwrap();
        assert_eq!(deserialized.offset.x, 0.0);
        assert_eq!(deserialized.offset.y, 0.0);
    }
}
