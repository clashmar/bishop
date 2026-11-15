/// Decodes PNG bytes into top-first RGBA pixels and dimensions.
pub(crate) fn decode_png(data: &[u8]) -> Result<(Vec<u8>, u32, u32), image::ImageError> {
    let rgba = image::load_from_memory(data)?.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((rgba.into_raw(), width, height))
}

#[cfg(test)]
mod tests {
    use super::decode_png;
    use image::{ImageEncoder, codecs::png::PngEncoder};

    #[test]
    fn png_decoder_preserves_rgba_dimensions_and_source_rows() -> Result<(), image::ImageError> {
        let pixels = [
            255, 0, 0, 255, 0, 255, 0, 255,
            0, 0, 255, 255, 255, 255, 0, 255,
        ];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded).write_image(&pixels, 2, 2, image::ExtendedColorType::Rgba8)?;

        let (decoded, width, height) = decode_png(&encoded)?;

        assert_eq!((width, height), (2, 2));
        assert_eq!(decoded, pixels);
        Ok(())
    }

    #[test]
    fn png_decoder_rejects_invalid_bytes() {
        assert!(decode_png(b"not a PNG").is_err());
    }
}
