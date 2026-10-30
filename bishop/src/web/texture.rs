use std::rc::Rc;

use glow::HasContext;

use super::gl_context::{GlContext, WebContext};
use crate::texture::png::decode_png;
use crate::texture::TextureLoader;
use crate::types::Texture2D;

/// A WebGL2 texture that keeps its originating context alive.
pub struct WebTexture {
    pub(crate) owner: Rc<GlContext>,
    handle: glow::Texture,
    width: u32,
    height: u32,
}

impl WebTexture {
    /// The WebGL2 texture name for backend-specific interop.
    pub fn handle(&self) -> glow::Texture {
        self.handle
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }
}

impl Drop for WebTexture {
    fn drop(&mut self) {
        if self.owner.make_current().is_ok() {
            unsafe { self.owner.gl.delete_texture(self.handle) };
        }
    }
}

impl TextureLoader for WebContext {
    fn load_texture_from_bytes(&self, data: &[u8]) -> Result<Texture2D, String> {
        let (pixels, width, height) = decode_png(data).map_err(|error| error.to_string())?;
        upload_rgba(&self.owner, &pixels, width, height)
    }

    fn load_texture_from_path(&self, path: &str) -> Result<Texture2D, String> {
        Err(format!(
            "browser texture path {path:?} is unavailable; load fetched PNG bytes instead"
        ))
    }

    fn empty_texture(&self) -> Texture2D {
        self.empty_texture.clone()
    }
}

pub(super) fn upload_rgba(
    owner: &Rc<GlContext>,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Texture2D, String> {
    owner.make_current()?;
    let width_gl = i32::try_from(width).map_err(|error| error.to_string())?;
    let height_gl = i32::try_from(height).map_err(|error| error.to_string())?;
    let gl = &owner.gl;
    let handle = unsafe { gl.create_texture()? };
    let result = unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(handle));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            width_gl,
            height_gl,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(pixels)),
        );
        let error = gl.get_error();
        gl.bind_texture(glow::TEXTURE_2D, None);
        if error == glow::NO_ERROR {
            Ok(())
        } else {
            Err(format!("WebGL2 RGBA texture upload failed: GL error {error}"))
        }
    };
    if let Err(error) = result {
        unsafe { gl.delete_texture(handle) };
        return Err(error);
    }
    Ok(Texture2D::from_web(WebTexture {
        owner: Rc::clone(owner),
        handle,
        width,
        height,
    }))
}
