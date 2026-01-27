//! Floating scalar image quantity.

use polyscope_core::quantity::{Quantity, QuantityKind};

use super::ImageOrigin;

/// A floating scalar image quantity (not attached to any structure).
///
/// Displays a 2D grid of scalar values using a colormap.
pub struct FloatingScalarImage {
    name: String,
    width: u32,
    height: u32,
    values: Vec<f32>,
    origin: ImageOrigin,
    enabled: bool,
    colormap_name: String,
    data_min: f32,
    data_max: f32,
}

impl FloatingScalarImage {
    /// Creates a new floating scalar image.
    pub fn new(name: impl Into<String>, width: u32, height: u32, values: Vec<f32>) -> Self {
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        Self {
            name: name.into(),
            width,
            height,
            values,
            origin: ImageOrigin::default(),
            enabled: true,
            colormap_name: "viridis".to_string(),
            data_min: min,
            data_max: max,
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

    /// Returns the scalar values.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
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

    /// Gets the colormap name.
    #[must_use]
    pub fn colormap_name(&self) -> &str {
        &self.colormap_name
    }

    /// Sets the colormap name.
    pub fn set_colormap(&mut self, name: impl Into<String>) -> &mut Self {
        self.colormap_name = name.into();
        self
    }

    /// Gets the data range minimum.
    #[must_use]
    pub fn data_min(&self) -> f32 {
        self.data_min
    }

    /// Gets the data range maximum.
    #[must_use]
    pub fn data_max(&self) -> f32 {
        self.data_max
    }

    /// Sets the data range.
    pub fn set_data_range(&mut self, min: f32, max: f32) -> &mut Self {
        self.data_min = min;
        self.data_max = max;
        self
    }

    /// Returns the pixel value at (x, y), accounting for image origin.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> f32 {
        let row = match self.origin {
            ImageOrigin::UpperLeft => y,
            ImageOrigin::LowerLeft => self.height - 1 - y,
        };
        self.values[(row * self.width + x) as usize]
    }
}

impl Quantity for FloatingScalarImage {
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
        QuantityKind::Other
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
        self.values.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_image_creation() {
        let values = vec![0.0, 0.5, 1.0, 1.5];
        let img = FloatingScalarImage::new("test", 2, 2, values);

        assert_eq!(img.name(), "test");
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.data_size(), 4);
        assert_eq!(img.data_min(), 0.0);
        assert_eq!(img.data_max(), 1.5);
        assert_eq!(img.kind(), QuantityKind::Other);
        assert!(img.is_enabled());
    }

    #[test]
    fn test_scalar_image_pixel_access() {
        // 2x2 image: [0, 1, 2, 3] row-major
        let values = vec![0.0, 1.0, 2.0, 3.0];
        let img = FloatingScalarImage::new("test", 2, 2, values);

        // UpperLeft (default): row 0 = top
        assert_eq!(img.pixel(0, 0), 0.0); // top-left
        assert_eq!(img.pixel(1, 0), 1.0); // top-right
        assert_eq!(img.pixel(0, 1), 2.0); // bottom-left
        assert_eq!(img.pixel(1, 1), 3.0); // bottom-right
    }

    #[test]
    fn test_scalar_image_lower_left_origin() {
        let values = vec![0.0, 1.0, 2.0, 3.0];
        let mut img = FloatingScalarImage::new("test", 2, 2, values);
        img.set_origin(ImageOrigin::LowerLeft);

        // LowerLeft: row 0 = bottom, so (0,0) maps to bottom-left = values[2]
        assert_eq!(img.pixel(0, 0), 2.0); // y=0 is bottom row
        assert_eq!(img.pixel(1, 0), 3.0);
        assert_eq!(img.pixel(0, 1), 0.0); // y=1 is top row
        assert_eq!(img.pixel(1, 1), 1.0);
    }

    #[test]
    fn test_scalar_image_setters() {
        let mut img = FloatingScalarImage::new("test", 2, 2, vec![0.0; 4]);

        img.set_colormap("blues");
        assert_eq!(img.colormap_name(), "blues");

        img.set_data_range(-1.0, 1.0);
        assert_eq!(img.data_min(), -1.0);
        assert_eq!(img.data_max(), 1.0);
    }
}
