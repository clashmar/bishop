use std::collections::{HashMap, hash_map::Entry};
use std::rc::Rc;

use super::draw::WebRenderer;
use super::gl_context::{GlContext, WebContext};
use super::texture::upload_rgba;
use crate::text::font_metrics::measure_text;
use crate::text::{Text, TextDimensions, TextParams};
use crate::types::{Color, Texture2D};

struct Glyph {
    texture: Option<Texture2D>,
    width: f32,
    height: f32,
    xmin: f32,
    ymin: f32,
    advance: f32,
}

pub(crate) struct WebText {
    font: fontdue::Font,
    glyphs: HashMap<(char, u32), Glyph>,
}

impl WebText {
    pub(crate) fn new() -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(
            include_bytes!("../fonts/gnf.regular.ttf") as &[u8],
            fontdue::FontSettings::default(),
        )
        .map_err(|error| error.to_string())?;
        Ok(Self {
            font,
            glyphs: HashMap::new(),
        })
    }

    pub(crate) fn measure(&self, text: &str, size: f32) -> TextDimensions {
        measure_text(&self.font, text, size)
    }

    fn glyph(&mut self, character: char, size: f32, owner: &Rc<GlContext>) -> Result<&Glyph, String> {
        let key = (character, size.to_bits());
        match self.glyphs.entry(key) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let (metrics, bitmap) = self.font.rasterize(character, size);
                let texture = if metrics.width == 0 || metrics.height == 0 {
                    None
                } else {
                    let rgba: Vec<u8> = bitmap
                        .into_iter()
                        .flat_map(|alpha| [255, 255, 255, alpha])
                        .collect();
                    Some(upload_rgba(owner, &rgba, metrics.width as u32, metrics.height as u32)?)
                };
                Ok(entry.insert(Glyph {
                    texture,
                    width: metrics.width as f32,
                    height: metrics.height as f32,
                    xmin: metrics.xmin as f32,
                    ymin: metrics.ymin as f32,
                    advance: metrics.advance_width,
                }))
            }
        }
    }

    pub(crate) fn draw(
        &mut self,
        renderer: &WebRenderer,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        params: &TextParams,
    ) -> Result<TextDimensions, String> {
        let dimensions = self.measure(text, size);
        let sx = params.font_scale * params.font_scale_aspect;
        let sy = params.font_scale;
        let width = dimensions.width * sx;
        let height = dimensions.height * sy;
        let ascent = dimensions.offset_y * sy;
        let pivot = [width * 0.5, height * 0.5];
        let center = [x + pivot[0], y - ascent + pivot[1]];
        let (sin, cos) = params.rotation.sin_cos();
        let mut cursor = 0.0;

        for character in text.chars() {
            let glyph = self.glyph(character, size, &renderer.owner)?;
            if let Some(texture) = &glyph.texture {
                let left = (cursor + glyph.xmin) * sx;
                let top = (dimensions.offset_y - glyph.height - glyph.ymin) * sy;
                let right = left + glyph.width * sx;
                let bottom = top + glyph.height * sy;
                let corners = [[left, top], [right, top], [right, bottom], [left, bottom]];
                let positions = corners.map(|[px, py]| {
                    let dx = px - pivot[0];
                    let dy = py - pivot[1];
                    [center[0] + dx * cos - dy * sin, center[1] + dx * sin + dy * cos]
                });
                renderer.draw_quad(
                    positions,
                    [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    texture.inner_web().handle(),
                    params.color,
                );
            }
            cursor += glyph.advance;
        }
        Ok(TextDimensions {
            width,
            height,
            offset_y: ascent,
        })
    }
}

impl Text for WebContext {
    fn draw_text(
        &mut self, text: &str, x: f32, y: f32, font_size: f32, color: Color,
    ) -> TextDimensions {
        let params = TextParams {
            color,
            ..Default::default()
        };
        match self.text.draw(&self.renderer, text, x, y, font_size, &params) {
            Ok(dimensions) => dimensions,
            Err(error) => {
                self.owner.record_error(error);
                self.text.measure(text, font_size)
            }
        }
    }

    fn draw_text_ex(&mut self, text: &str, x: f32, y: f32, params: TextParams) -> TextDimensions {
        let dimensions = self.text.measure(text, params.font_size as f32);
        let scaled = TextDimensions {
            width: dimensions.width * params.font_scale * params.font_scale_aspect,
            height: dimensions.height * params.font_scale,
            offset_y: dimensions.offset_y * params.font_scale,
        };
        if let Err(error) = self.text.draw(&self.renderer, text, x, y, params.font_size as f32, &params) {
            self.owner.record_error(error);
        }
        scaled
    }

    fn measure_text(&self, text: &str, font_size: f32) -> TextDimensions {
        self.text.measure(text, font_size)
    }
}
