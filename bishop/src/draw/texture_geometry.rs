use crate::draw::DrawTextureParams;
use crate::types::{Rect, Vec2};

pub(crate) struct SpriteQuad {
    pub positions: [[f32; 2]; 4],
    pub uv: [[f32; 2]; 4],
}

pub(crate) fn sprite_quad(
    texture_size: Vec2,
    position: Vec2,
    params: &DrawTextureParams,
) -> SpriteQuad {
    let source = params
        .source
        .unwrap_or(Rect::new(0.0, 0.0, texture_size.x, texture_size.y));
    let size = params.dest_size.unwrap_or(source.size());
    let mut u0 = source.x / texture_size.x;
    let mut u1 = source.right() / texture_size.x;
    if params.flip_x {
        std::mem::swap(&mut u0, &mut u1);
    }
    let mut v0 = source.y / texture_size.y;
    let mut v1 = source.bottom() / texture_size.y;
    if params.flip_y {
        std::mem::swap(&mut v0, &mut v1);
    }

    let corners = [[0.0, 0.0], [size.x, 0.0], [size.x, size.y], [0.0, size.y]];
    let positions = if params.rotation != 0.0 {
        let pivot = params.pivot.unwrap_or(size * 0.5);
        let (sin, cos) = params.rotation.sin_cos();
        corners.map(|[x, y]| {
            let dx = x - pivot.x;
            let dy = y - pivot.y;
            [
                position.x + pivot.x + dx * cos - dy * sin,
                position.y + pivot.y + dx * sin + dy * cos,
            ]
        })
    } else {
        corners.map(|[x, y]| [position.x + x, position.y + y])
    };

    SpriteQuad {
        positions,
        uv: [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
    }
}

#[cfg(test)]
mod tests {
    use super::sprite_quad;
    use crate::draw::DrawTextureParams;
    use crate::types::{Rect, Vec2};

    #[test]
    fn cropped_flipped_sprite_uses_frame_uvs_and_cropped_size() {
        let params = DrawTextureParams {
            source: Some(Rect::new(6.0, 0.0, 6.0, 16.0)),
            flip_x: true,
            ..Default::default()
        };

        let quad = sprite_quad(Vec2::new(30.0, 16.0), Vec2::new(10.0, 8.0), &params);

        assert_eq!(quad.uv[0], [0.4, 0.0]);
        assert_eq!(quad.uv[1], [0.2, 0.0]);
        assert_eq!(quad.positions[0], [10.0, 8.0]);
        assert_eq!(quad.positions[2], [16.0, 24.0]);
    }

    #[test]
    fn vertical_flip_reverses_frame_v_coordinates() {
        let params = DrawTextureParams {
            source: Some(Rect::new(6.0, 0.0, 6.0, 16.0)),
            flip_y: true,
            ..Default::default()
        };

        let quad = sprite_quad(Vec2::new(30.0, 16.0), Vec2::ZERO, &params);

        assert_eq!(quad.uv[0], [0.2, 1.0]);
        assert_eq!(quad.uv[2], [0.4, 0.0]);
    }

    #[test]
    fn rotation_uses_noncentral_pivot_in_destination_space() {
        let params = DrawTextureParams {
            dest_size: Some(Vec2::new(4.0, 2.0)),
            rotation: std::f32::consts::FRAC_PI_2,
            pivot: Some(Vec2::new(1.0, 0.0)),
            ..Default::default()
        };

        let quad = sprite_quad(Vec2::new(6.0, 16.0), Vec2::new(10.0, 8.0), &params);
        let expected = [[11.0, 7.0], [11.0, 11.0], [9.0, 11.0], [9.0, 7.0]];

        for (actual, expected) in quad.positions.into_iter().zip(expected) {
            assert!((actual[0] - expected[0]).abs() < 0.00001);
            assert!((actual[1] - expected[1]).abs() < 0.00001);
        }
    }
}
