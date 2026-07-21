use bishop::prelude::Vec2;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum_macros::EnumIter;

/// Default width and height for colliders without a sprite or animation reference.
pub const DEFAULT_COLLIDER_DIMENSION: f32 = 16.0;

#[ecs_component]
#[serde_as]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Collider {
    pub shape: ColliderShape,
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub offset: Vec2,
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
        };
        let ron = ron::ser::to_string(&collider).unwrap();
        let deserialized: Collider = ron::de::from_str(&ron).unwrap();
        assert_eq!(deserialized.offset.x, 3.0);
        assert_eq!(deserialized.offset.y, -4.0);
    }

    #[test]
    fn collider_offset_deserialization_missing_field() {
        let ron = r#"Collider(shape: Aabb(width: 8.0, height: 12.0))"#;
        let deserialized: Collider = ron::de::from_str(ron).unwrap();
        assert_eq!(deserialized.offset.x, 0.0);
        assert_eq!(deserialized.offset.y, 0.0);
    }
}
