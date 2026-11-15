use super::TextDimensions;

pub(crate) fn measure_text(font: &fontdue::Font, text: &str, font_size: f32) -> TextDimensions {
    let width = text
        .chars()
        .map(|character| font.metrics(character, font_size).advance_width)
        .sum();
    let (ascent, descent) = font
        .horizontal_line_metrics(font_size)
        .map(|metrics| (metrics.ascent, -metrics.descent))
        .unwrap_or((font_size, 0.0));
    TextDimensions {
        width,
        height: ascent + descent,
        offset_y: ascent,
    }
}

#[cfg(test)]
mod tests {
    use super::measure_text;

    #[test]
    fn fontdue_measurement_matches_embedded_font_advances_and_baseline() -> Result<(), String> {
        let font = fontdue::Font::from_bytes(
            include_bytes!("../fonts/gnf.regular.ttf") as &[u8],
            fontdue::FontSettings::default(),
        )
        .map_err(|error| error.to_string())?;
        let line = font.horizontal_line_metrics(16.0)
            .ok_or_else(|| "embedded font has no line metrics".to_owned())?;

        let measured = measure_text(&font, "Bb", 16.0);

        assert_eq!(
            measured.width,
            font.metrics('B', 16.0).advance_width + font.metrics('b', 16.0).advance_width,
        );
        assert_eq!(measured.height, line.ascent - line.descent);
        assert_eq!(measured.offset_y, line.ascent);
        Ok(())
    }
}
