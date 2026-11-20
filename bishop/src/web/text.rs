use std::collections::{HashMap, hash_map::Entry};
use std::rc::Rc;

use super::draw::WebRenderer;
use super::gl_context::{GlContext, WebContext};
use super::texture::upload_rgba;
use crate::text::font_metrics::measure_text;
use crate::text::{FontId, Text, TextDimensions, TextParams};
use crate::types::{Color, Texture2D};

struct Glyph {
    texture: Option<Texture2D>,
    width: f32,
    height: f32,
    xmin: f32,
    ymin: f32,
    advance: f32,
}

type GlyphCache = HashMap<(Option<FontId>, char, u32), Glyph>;

pub(crate) struct WebText {
    fonts: HashMap<Option<FontId>, fontdue::Font>,
    glyphs: GlyphCache,
}

impl WebText {
    pub(crate) fn new() -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(
            include_bytes!("../fonts/gnf.regular.ttf") as &[u8],
            fontdue::FontSettings::default(),
        )
        .map_err(|error| error.to_string())?;
        Ok(Self {
            fonts: HashMap::from([(None, font)]),
            glyphs: HashMap::new(),
        })
    }

    pub(crate) fn register_font(&mut self, id: FontId, data: &[u8]) -> Result<(), String> {
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
            .map_err(|error| error.to_string())?;
        self.fonts.insert(Some(id), font);
        self.glyphs.retain(|(font_id, _, _), _| *font_id != Some(id));
        Ok(())
    }

    pub(crate) fn measure(&self, text: &str, size: f32) -> TextDimensions {
        self.measure_with_font(text, size, None)
    }

    pub(crate) fn measure_with_font(
        &self,
        text: &str,
        size: f32,
        font_id: Option<FontId>,
    ) -> TextDimensions {
        let font_id = font_id.filter(|id| self.fonts.contains_key(&Some(*id)));
        let font = self
            .fonts
            .get(&font_id)
            .or_else(|| self.fonts.get(&None))
            .expect("default font is always registered");
        measure_text(font, text, size)
    }

    fn glyph(
        &mut self,
        character: char,
        size: f32,
        font_id: Option<FontId>,
        owner: &Rc<GlContext>,
    ) -> Result<&Glyph, String> {
        let font_id = font_id.filter(|id| self.fonts.contains_key(&Some(*id)));
        let key = (font_id, character, size.to_bits());
        match self.glyphs.entry(key) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let font = self
                    .fonts
                    .get(&font_id)
                    .or_else(|| self.fonts.get(&None))
                    .expect("default font is always registered");
                let (metrics, bitmap) = font.rasterize(character, size);
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
        raster_scale: f32,
    ) -> Result<TextDimensions, String> {
        let raster_scale = raster_scale.max(0.01);
        let raster_size = size * params.font_scale * raster_scale;
        let dimensions = self.measure_with_font(text, raster_size, params.font);
        let sx = params.font_scale_aspect / raster_scale;
        let sy = 1.0 / raster_scale;
        let width = dimensions.width * sx;
        let height = dimensions.height * sy;
        let ascent = dimensions.offset_y * sy;
        let pivot = [width * 0.5, height * 0.5];
        let center = [x + pivot[0], y - ascent + pivot[1]];
        let (sin, cos) = params.rotation.sin_cos();
        let mut cursor = 0.0;

        for character in text.chars() {
            let glyph = self.glyph(character, raster_size, params.font, &renderer.owner)?;
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

#[cfg(test)]
mod tests {
    use super::WebText;
    use crate::text::FontId;

    #[test]
    fn registered_font_is_used_for_web_measurement() -> Result<(), String> {
        let mut text = WebText::new()?;
        let font_id = FontId(23);
        text.register_font(font_id, include_bytes!("../fonts/gnf.regular.ttf"))?;

        let default = text.measure("Bishop", 18.0);
        let selected = text.measure_with_font("Bishop", 18.0, Some(font_id));

        assert!(text.fonts.contains_key(&Some(font_id)));
        assert_eq!(selected.width, default.width);
        assert_eq!(selected.height, default.height);
        assert_eq!(selected.offset_y, default.offset_y);
        let missing = text.measure_with_font("Bishop", 18.0, Some(FontId(999)));
        assert_eq!(missing.width, default.width);
        assert_eq!(missing.height, default.height);
        assert_eq!(missing.offset_y, default.offset_y);
        assert!(text.register_font(FontId(999), b"not a font").is_err());
        Ok(())
    }
}

impl Text for WebContext {
    fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> TextDimensions {
        let params = TextParams {
            color,
            ..Default::default()
        };
        match self.text.draw(
            &self.renderer,
            text,
            x,
            y,
            font_size,
            &params,
            self.text_raster_scale(),
        ) {
            Ok(dimensions) => dimensions,
            Err(error) => {
                self.owner.record_error(error);
                self.text.measure(text, font_size)
            }
        }
    }

    fn draw_text_ex(&mut self, text: &str, x: f32, y: f32, params: TextParams) -> TextDimensions {
        let scaled = self.measure_text_ex(text, &params);
        if let Err(error) = self.text.draw(
            &self.renderer,
            text,
            x,
            y,
            params.font_size as f32,
            &params,
            self.text_raster_scale(),
        ) {
            self.owner.record_error(error);
        }
        scaled
    }

    fn measure_text(&self, text: &str, font_size: f32) -> TextDimensions {
        self.text.measure(text, font_size)
    }

    fn measure_text_ex(&self, text: &str, params: &TextParams) -> TextDimensions {
        self.text
            .measure_with_font(text, params.font_size as f32, params.font)
            .scaled(
                params.font_scale * params.font_scale_aspect,
                params.font_scale,
            )
    }

    fn register_font(&mut self, id: FontId, ttf_data: &[u8]) -> Result<(), String> {
        self.text.register_font(id, ttf_data)
    }
}
