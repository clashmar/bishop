//! Window configuration types for bishop applications.

/// Window configuration for bishop applications.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title displayed in the title bar.
    pub title: String,
    /// Initial window width in logical pixels.
    pub width: u32,
    /// Initial window height in logical pixels.
    pub height: u32,
    /// Whether to start in fullscreen mode.
    pub fullscreen: bool,
    /// Whether the window can be resized.
    pub resizable: bool,
    /// Optional window icon.
    pub icon: Option<WindowIcon>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Bishop Application".to_string(),
            width: 800,
            height: 600,
            fullscreen: false,
            resizable: true,
            icon: None,
        }
    }
}

impl WindowConfig {
    /// Creates a new window configuration with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Sets the window dimensions.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets whether the window starts in fullscreen mode.
    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// Sets whether the window can be resized.
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Sets the window icon.
    pub fn with_icon(mut self, icon: WindowIcon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// Window icon data.
#[derive(Debug, Clone)]
pub enum WindowIcon {
    /// PNG-encoded icon data (will be decoded at runtime).
    Png(Vec<u8>),
    /// Pre-decoded RGBA icon data with dimensions.
    Rgba {
        /// Small icon (typically 16x16 or 32x32).
        small: Option<IconData>,
        /// Medium icon (typically 48x48).
        medium: Option<IconData>,
        /// Large icon (typically 64x64 or higher).
        large: Option<IconData>,
    },
}

/// Raw RGBA icon data with dimensions.
#[derive(Debug, Clone)]
pub struct IconData {
    /// RGBA pixel data (4 bytes per pixel).
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

impl IconData {
    /// Creates new icon data from RGBA pixels.
    pub fn new(rgba: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            rgba,
            width,
            height,
        }
    }
}
