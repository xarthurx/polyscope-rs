//! Render image floating quantities for depth-composited external renders.

use glam::Vec3;
use polyscope_core::quantity::{Quantity, QuantityKind};

use super::ImageOrigin;

/// A depth render image (geometry from an external renderer).
///
/// Stores per-pixel radial depth values and optional world-space normals.
/// Used for depth-compositing external renders with the polyscope scene.
pub struct FloatingDepthRenderImage {
    name: String,
    width: u32,
    height: u32,
    depths: Vec<f32>,            // Radial distance from camera
    normals: Option<Vec<Vec3>>,  // Optional world-space normals
    origin: ImageOrigin,
    enabled: bool,
}

impl FloatingDepthRenderImage {
    /// Creates a new depth render image.
    pub fn new(
        name: impl Into<String>,
        width: u32,
        height: u32,
        depths: Vec<f32>,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            height,
            depths,
            normals: None,
            origin: ImageOrigin::default(),
            enabled: true,
        }
    }

    /// Sets per-pixel world-space normals.
    pub fn set_normals(&mut self, normals: Vec<Vec3>) -> &mut Self {
        self.normals = Some(normals);
        self
    }

    /// Returns the image width.
    #[must_use]
    pub fn width(&self) -> u32 { self.width }

    /// Returns the image height.
    #[must_use]
    pub fn height(&self) -> u32 { self.height }

    /// Returns the depth values.
    #[must_use]
    pub fn depths(&self) -> &[f32] { &self.depths }

    /// Returns the normals, if set.
    #[must_use]
    pub fn normals(&self) -> Option<&[Vec3]> {
        self.normals.as_deref()
    }

    /// Gets the image origin.
    #[must_use]
    pub fn origin(&self) -> ImageOrigin { self.origin }

    /// Sets the image origin.
    pub fn set_origin(&mut self, origin: ImageOrigin) -> &mut Self {
        self.origin = origin;
        self
    }

    /// Returns the depth at pixel (x, y), accounting for image origin.
    #[must_use]
    pub fn depth_at(&self, x: u32, y: u32) -> f32 {
        let row = match self.origin {
            ImageOrigin::UpperLeft => y,
            ImageOrigin::LowerLeft => self.height - 1 - y,
        };
        self.depths[(row * self.width + x) as usize]
    }

    /// Returns whether a pixel has valid depth (not infinity/NaN).
    #[must_use]
    pub fn has_depth(&self, x: u32, y: u32) -> bool {
        let d = self.depth_at(x, y);
        d.is_finite() && d > 0.0
    }
}

impl Quantity for FloatingDepthRenderImage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { "" }
    fn kind(&self) -> QuantityKind { QuantityKind::Other }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn data_size(&self) -> usize { self.depths.len() }
}

/// A color render image (colored geometry from an external renderer).
///
/// Stores per-pixel depth, color, and optional normals for depth-compositing
/// externally rendered content with polyscope's scene.
pub struct FloatingColorRenderImage {
    name: String,
    width: u32,
    height: u32,
    depths: Vec<f32>,
    colors: Vec<Vec3>,           // Per-pixel RGB
    normals: Option<Vec<Vec3>>,
    origin: ImageOrigin,
    enabled: bool,
}

impl FloatingColorRenderImage {
    /// Creates a new color render image.
    pub fn new(
        name: impl Into<String>,
        width: u32,
        height: u32,
        depths: Vec<f32>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            height,
            depths,
            colors,
            normals: None,
            origin: ImageOrigin::default(),
            enabled: true,
        }
    }

    /// Sets per-pixel world-space normals.
    pub fn set_normals(&mut self, normals: Vec<Vec3>) -> &mut Self {
        self.normals = Some(normals);
        self
    }

    /// Returns the image width.
    #[must_use]
    pub fn width(&self) -> u32 { self.width }

    /// Returns the image height.
    #[must_use]
    pub fn height(&self) -> u32 { self.height }

    /// Returns the depth values.
    #[must_use]
    pub fn depths(&self) -> &[f32] { &self.depths }

    /// Returns the pixel colors.
    #[must_use]
    pub fn colors(&self) -> &[Vec3] { &self.colors }

    /// Returns the normals, if set.
    #[must_use]
    pub fn normals(&self) -> Option<&[Vec3]> {
        self.normals.as_deref()
    }

    /// Gets the image origin.
    #[must_use]
    pub fn origin(&self) -> ImageOrigin { self.origin }

    /// Sets the image origin.
    pub fn set_origin(&mut self, origin: ImageOrigin) -> &mut Self {
        self.origin = origin;
        self
    }

    /// Returns the depth at pixel (x, y).
    #[must_use]
    pub fn depth_at(&self, x: u32, y: u32) -> f32 {
        let row = match self.origin {
            ImageOrigin::UpperLeft => y,
            ImageOrigin::LowerLeft => self.height - 1 - y,
        };
        self.depths[(row * self.width + x) as usize]
    }

    /// Returns the color at pixel (x, y).
    #[must_use]
    pub fn color_at(&self, x: u32, y: u32) -> Vec3 {
        let row = match self.origin {
            ImageOrigin::UpperLeft => y,
            ImageOrigin::LowerLeft => self.height - 1 - y,
        };
        self.colors[(row * self.width + x) as usize]
    }
}

impl Quantity for FloatingColorRenderImage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { "" }
    fn kind(&self) -> QuantityKind { QuantityKind::Color }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn data_size(&self) -> usize { self.colors.len() }
}

/// A raw color render image (direct display, no shading).
///
/// Stores per-pixel colors for direct display without depth compositing
/// or lighting. Suitable for pre-lit content or UI overlays.
pub struct FloatingRawColorImage {
    name: String,
    width: u32,
    height: u32,
    colors: Vec<Vec3>,
    origin: ImageOrigin,
    enabled: bool,
}

impl FloatingRawColorImage {
    /// Creates a new raw color render image.
    pub fn new(
        name: impl Into<String>,
        width: u32,
        height: u32,
        colors: Vec<Vec3>,
    ) -> Self {
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
    pub fn width(&self) -> u32 { self.width }

    /// Returns the image height.
    #[must_use]
    pub fn height(&self) -> u32 { self.height }

    /// Returns the pixel colors.
    #[must_use]
    pub fn colors(&self) -> &[Vec3] { &self.colors }

    /// Gets the image origin.
    #[must_use]
    pub fn origin(&self) -> ImageOrigin { self.origin }

    /// Sets the image origin.
    pub fn set_origin(&mut self, origin: ImageOrigin) -> &mut Self {
        self.origin = origin;
        self
    }

    /// Returns the color at pixel (x, y).
    #[must_use]
    pub fn color_at(&self, x: u32, y: u32) -> Vec3 {
        let row = match self.origin {
            ImageOrigin::UpperLeft => y,
            ImageOrigin::LowerLeft => self.height - 1 - y,
        };
        self.colors[(row * self.width + x) as usize]
    }
}

impl Quantity for FloatingRawColorImage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { "" }
    fn kind(&self) -> QuantityKind { QuantityKind::Color }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn data_size(&self) -> usize { self.colors.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_render_image_creation() {
        let depths = vec![1.0, 2.0, 3.0, 4.0];
        let img = FloatingDepthRenderImage::new("depth", 2, 2, depths);

        assert_eq!(img.name(), "depth");
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.data_size(), 4);
        assert!(img.normals().is_none());
    }

    #[test]
    fn test_depth_render_image_pixel_access() {
        let depths = vec![1.0, 2.0, 3.0, 4.0];
        let img = FloatingDepthRenderImage::new("depth", 2, 2, depths);

        assert_eq!(img.depth_at(0, 0), 1.0);
        assert_eq!(img.depth_at(1, 0), 2.0);
        assert_eq!(img.depth_at(0, 1), 3.0);
        assert_eq!(img.depth_at(1, 1), 4.0);
    }

    #[test]
    fn test_depth_render_image_has_depth() {
        let depths = vec![1.0, f32::INFINITY, 0.0, -1.0];
        let img = FloatingDepthRenderImage::new("depth", 2, 2, depths);

        assert!(img.has_depth(0, 0));   // 1.0 — valid
        assert!(!img.has_depth(1, 0));  // inf — invalid
        assert!(!img.has_depth(0, 1));  // 0.0 — invalid (not > 0)
        assert!(!img.has_depth(1, 1));  // -1.0 — invalid
    }

    #[test]
    fn test_depth_render_image_with_normals() {
        let depths = vec![1.0; 4];
        let normals = vec![Vec3::Z; 4];
        let mut img = FloatingDepthRenderImage::new("depth", 2, 2, depths);
        img.set_normals(normals);

        assert!(img.normals().is_some());
        assert_eq!(img.normals().unwrap().len(), 4);
    }

    #[test]
    fn test_color_render_image_creation() {
        let depths = vec![1.0, 2.0, 3.0, 4.0];
        let colors = vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE];
        let img = FloatingColorRenderImage::new("colored", 2, 2, depths, colors);

        assert_eq!(img.name(), "colored");
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.data_size(), 4);
        assert_eq!(img.kind(), QuantityKind::Color);
    }

    #[test]
    fn test_color_render_image_pixel_access() {
        let depths = vec![1.0, 2.0, 3.0, 4.0];
        let colors = vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE];
        let img = FloatingColorRenderImage::new("colored", 2, 2, depths, colors);

        assert_eq!(img.depth_at(0, 0), 1.0);
        assert_eq!(img.color_at(0, 0), Vec3::X);
        assert_eq!(img.color_at(1, 1), Vec3::ONE);
    }

    #[test]
    fn test_raw_color_image_creation() {
        let colors = vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE];
        let img = FloatingRawColorImage::new("raw", 2, 2, colors);

        assert_eq!(img.name(), "raw");
        assert_eq!(img.data_size(), 4);
        assert_eq!(img.kind(), QuantityKind::Color);
    }

    #[test]
    fn test_raw_color_image_pixel_access() {
        let colors = vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE];
        let img = FloatingRawColorImage::new("raw", 2, 2, colors);

        assert_eq!(img.color_at(0, 0), Vec3::X);
        assert_eq!(img.color_at(1, 0), Vec3::Y);
        assert_eq!(img.color_at(0, 1), Vec3::Z);
        assert_eq!(img.color_at(1, 1), Vec3::ONE);
    }
}
