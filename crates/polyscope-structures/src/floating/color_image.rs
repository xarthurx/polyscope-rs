//! Floating color image quantity.

use glam::Vec3;
use polyscope_core::quantity::{Quantity, QuantityKind};

use super::ImageOrigin;

/// A floating color image quantity (not attached to any structure).
///
/// Displays a 2D grid of RGB colors directly.
pub struct FloatingColorImage {
    name: String,
    width: u32,
    height: u32,
    colors: Vec<Vec3>, // RGB per pixel
    origin: ImageOrigin,
    enabled: bool,
}

impl FloatingColorImage {
    /// Creates a new floating color image.
    pub fn new(name: impl Into<String>, width: u32, height: u32, colors: Vec<Vec3>) -> Self {
        Self {
            name: name.into(),
            width,
            height,
            colors,
            origin: ImageOrigin::default(),
            enabled: true,
        }
    }

    /// Returns the image width.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the image height.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the pixel colors.
    #[must_use]
    pub fn colors(&self) -> &[Vec3] {
        &self.colors
    }

    /// Gets the image origin.
    #[must_use]
    pub fn origin(&self) -> ImageOrigin {
        self.origin
    }

    /// Sets the image origin.
    pub fn set_origin(&mut self, origin: ImageOrigin) -> &mut Self {
        self.origin = origin;
        self
    }

    /// Returns the pixel color at (x, y), accounting for image origin.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Vec3 {
        let row = match self.origin {
            ImageOrigin::UpperLeft => y,
            ImageOrigin::LowerLeft => self.height - 1 - y,
        };
        self.colors[(row * self.width + x) as usize]
    }
}

impl Quantity for FloatingColorImage {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn name(&self) -> &str {
        &self.name
    }
    #[allow(clippy::unnecessary_literal_bound)]
    fn structure_name(&self) -> &str {
        "" // No parent structure
    }
    fn kind(&self) -> QuantityKind {
        QuantityKind::Color
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn data_size(&self) -> usize {
        self.colors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_image_creation() {
        let colors = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];
        let img = FloatingColorImage::new("test", 2, 2, colors);

        assert_eq!(img.name(), "test");
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.data_size(), 4);
        assert_eq!(img.kind(), QuantityKind::Color);
    }

    #[test]
    fn test_color_image_pixel_access() {
        let colors = vec![
            Vec3::new(1.0, 0.0, 0.0), // (0,0) top-left red
            Vec3::new(0.0, 1.0, 0.0), // (1,0) top-right green
            Vec3::new(0.0, 0.0, 1.0), // (0,1) bottom-left blue
            Vec3::new(1.0, 1.0, 1.0), // (1,1) bottom-right white
        ];
        let img = FloatingColorImage::new("test", 2, 2, colors);

        assert_eq!(img.pixel(0, 0), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(img.pixel(1, 0), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(img.pixel(0, 1), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(img.pixel(1, 1), Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_color_image_lower_left_origin() {
        let colors = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];
        let mut img = FloatingColorImage::new("test", 2, 2, colors);
        img.set_origin(ImageOrigin::LowerLeft);

        // LowerLeft: y=0 maps to bottom row (index 2,3)
        assert_eq!(img.pixel(0, 0), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(img.pixel(1, 0), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(img.pixel(0, 1), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(img.pixel(1, 1), Vec3::new(0.0, 1.0, 0.0));
    }
}
