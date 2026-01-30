//! Parameterization (UV) quantities for surface meshes.

use glam::{Vec2, Vec3, Vec4};
use polyscope_core::quantity::{
    FaceQuantity, ParamCoordsType, ParamVizStyle, Quantity, QuantityKind, VertexQuantity,
};

/// Simple HSV to RGB conversion helper.
#[allow(clippy::many_single_char_names)]
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Vec3::new(r + m, g + m, b + m)
}

/// A vertex parameterization (UV) quantity on a surface mesh.
pub struct MeshVertexParameterizationQuantity {
    name: String,
    structure_name: String,
    coords: Vec<Vec2>,
    enabled: bool,
    // Visualization parameters
    style: ParamVizStyle,
    coords_type: ParamCoordsType,
    checker_size: f32,
    checker_colors: [Vec3; 2],
    grid_line_width: f32,
}

impl MeshVertexParameterizationQuantity {
    /// Creates a new vertex parameterization quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        coords: Vec<Vec2>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            coords,
            enabled: false,
            style: ParamVizStyle::default(),
            coords_type: ParamCoordsType::default(),
            checker_size: 0.1,
            checker_colors: [Vec3::new(1.0, 0.4, 0.4), Vec3::new(0.4, 0.4, 1.0)],
            grid_line_width: 0.02,
        }
    }

    /// Returns the UV coordinates.
    #[must_use]
    pub fn coords(&self) -> &[Vec2] {
        &self.coords
    }

    /// Gets the visualization style.
    #[must_use]
    pub fn style(&self) -> ParamVizStyle {
        self.style
    }

    /// Sets the visualization style.
    pub fn set_style(&mut self, style: ParamVizStyle) -> &mut Self {
        self.style = style;
        self
    }

    /// Gets the coordinate type.
    #[must_use]
    pub fn coords_type(&self) -> ParamCoordsType {
        self.coords_type
    }

    /// Sets the coordinate type.
    pub fn set_coords_type(&mut self, ct: ParamCoordsType) -> &mut Self {
        self.coords_type = ct;
        self
    }

    /// Gets the checker size.
    #[must_use]
    pub fn checker_size(&self) -> f32 {
        self.checker_size
    }

    /// Sets the checker size.
    pub fn set_checker_size(&mut self, size: f32) -> &mut Self {
        self.checker_size = size;
        self
    }

    /// Gets the checker colors.
    #[must_use]
    pub fn checker_colors(&self) -> [Vec3; 2] {
        self.checker_colors
    }

    /// Sets the checker colors.
    pub fn set_checker_colors(&mut self, colors: [Vec3; 2]) -> &mut Self {
        self.checker_colors = colors;
        self
    }

    /// Compute per-vertex colors based on the current visualization style.
    #[must_use]
    pub fn compute_colors(&self) -> Vec<Vec4> {
        match self.style {
            ParamVizStyle::Checker => self.compute_checker_colors(),
            ParamVizStyle::Grid => self.compute_grid_colors(),
            ParamVizStyle::LocalCheck => self.compute_local_check_colors(),
            ParamVizStyle::LocalRad => self.compute_local_rad_colors(),
        }
    }

    fn compute_checker_colors(&self) -> Vec<Vec4> {
        self.coords
            .iter()
            .map(|uv| {
                let u_cell = (uv.x / self.checker_size).floor() as i32;
                let v_cell = (uv.y / self.checker_size).floor() as i32;
                if (u_cell + v_cell) % 2 == 0 {
                    self.checker_colors[0].extend(1.0)
                } else {
                    self.checker_colors[1].extend(1.0)
                }
            })
            .collect()
    }

    fn compute_grid_colors(&self) -> Vec<Vec4> {
        self.coords
            .iter()
            .map(|uv| {
                let u_frac = (uv.x / self.checker_size).fract().abs();
                let v_frac = (uv.y / self.checker_size).fract().abs();
                let on_line = u_frac < self.grid_line_width
                    || u_frac > (1.0 - self.grid_line_width)
                    || v_frac < self.grid_line_width
                    || v_frac > (1.0 - self.grid_line_width);
                if on_line {
                    self.checker_colors[1].extend(1.0)
                } else {
                    self.checker_colors[0].extend(1.0)
                }
            })
            .collect()
    }

    fn compute_local_check_colors(&self) -> Vec<Vec4> {
        self.coords
            .iter()
            .map(|uv| {
                let r = uv.length();
                let angle = uv.y.atan2(uv.x);
                let hue = (angle / std::f32::consts::TAU + 1.0) % 1.0;
                let base = hsv_to_rgb(hue, 0.7, 0.9);
                let u_cell = (uv.x / self.checker_size).floor() as i32;
                let v_cell = (uv.y / self.checker_size).floor() as i32;
                let dim = if (u_cell + v_cell) % 2 == 0 { 1.0 } else { 0.6 };
                (base * dim * (1.0 - (-r * 2.0).exp() * 0.5)).extend(1.0)
            })
            .collect()
    }

    fn compute_local_rad_colors(&self) -> Vec<Vec4> {
        self.coords
            .iter()
            .map(|uv| {
                let r = uv.length();
                let angle = uv.y.atan2(uv.x);
                let hue = (angle / std::f32::consts::TAU + 1.0) % 1.0;
                let base = hsv_to_rgb(hue, 0.7, 0.9);
                let stripe = f32::from(u8::from((r / self.checker_size).floor() as i32 % 2 == 0));
                let dim = 0.6 + 0.4 * stripe;
                (base * dim).extend(1.0)
            })
            .collect()
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_parameterization_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.style,
            &mut self.checker_size,
            &mut self.checker_colors,
        )
    }
}

impl Quantity for MeshVertexParameterizationQuantity {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn structure_name(&self) -> &str {
        &self.structure_name
    }

    fn kind(&self) -> QuantityKind {
        QuantityKind::Parameterization
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
        self.coords.len()
    }
}

impl VertexQuantity for MeshVertexParameterizationQuantity {}

/// A corner (per-face-vertex) parameterization quantity.
/// Used when UV islands are disconnected (different UV at shared vertices).
pub struct MeshCornerParameterizationQuantity {
    name: String,
    structure_name: String,
    coords: Vec<Vec2>, // One per corner (3 * num_triangles for triangle meshes)
    enabled: bool,
    style: ParamVizStyle,
    coords_type: ParamCoordsType,
    checker_size: f32,
    checker_colors: [Vec3; 2],
    grid_line_width: f32,
}

impl MeshCornerParameterizationQuantity {
    /// Creates a new corner parameterization quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        coords: Vec<Vec2>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            coords,
            enabled: false,
            style: ParamVizStyle::default(),
            coords_type: ParamCoordsType::default(),
            checker_size: 0.1,
            checker_colors: [Vec3::new(1.0, 0.4, 0.4), Vec3::new(0.4, 0.4, 1.0)],
            grid_line_width: 0.02,
        }
    }

    /// Returns the UV coordinates.
    #[must_use]
    pub fn coords(&self) -> &[Vec2] {
        &self.coords
    }

    /// Gets the visualization style.
    #[must_use]
    pub fn style(&self) -> ParamVizStyle {
        self.style
    }

    /// Sets the visualization style.
    pub fn set_style(&mut self, style: ParamVizStyle) -> &mut Self {
        self.style = style;
        self
    }

    /// Gets the coordinate type.
    #[must_use]
    pub fn coords_type(&self) -> ParamCoordsType {
        self.coords_type
    }

    /// Sets the coordinate type.
    pub fn set_coords_type(&mut self, ct: ParamCoordsType) -> &mut Self {
        self.coords_type = ct;
        self
    }

    /// Gets the checker size.
    #[must_use]
    pub fn checker_size(&self) -> f32 {
        self.checker_size
    }

    /// Sets the checker size.
    pub fn set_checker_size(&mut self, size: f32) -> &mut Self {
        self.checker_size = size;
        self
    }

    /// Gets the checker colors.
    #[must_use]
    pub fn checker_colors(&self) -> [Vec3; 2] {
        self.checker_colors
    }

    /// Sets the checker colors.
    pub fn set_checker_colors(&mut self, colors: [Vec3; 2]) -> &mut Self {
        self.checker_colors = colors;
        self
    }

    /// Compute per-corner colors based on the current visualization style.
    /// Returns one color per corner (same length as self.coords).
    #[must_use]
    pub fn compute_colors(&self) -> Vec<Vec4> {
        self.coords
            .iter()
            .map(|uv| match self.style {
                ParamVizStyle::Checker => {
                    let u_cell = (uv.x / self.checker_size).floor() as i32;
                    let v_cell = (uv.y / self.checker_size).floor() as i32;
                    if (u_cell + v_cell) % 2 == 0 {
                        self.checker_colors[0].extend(1.0)
                    } else {
                        self.checker_colors[1].extend(1.0)
                    }
                }
                ParamVizStyle::Grid => {
                    let u_frac = (uv.x / self.checker_size).fract().abs();
                    let v_frac = (uv.y / self.checker_size).fract().abs();
                    let on_line = u_frac < self.grid_line_width
                        || u_frac > (1.0 - self.grid_line_width)
                        || v_frac < self.grid_line_width
                        || v_frac > (1.0 - self.grid_line_width);
                    if on_line {
                        self.checker_colors[1].extend(1.0)
                    } else {
                        self.checker_colors[0].extend(1.0)
                    }
                }
                ParamVizStyle::LocalCheck => {
                    let angle = uv.y.atan2(uv.x);
                    let hue = (angle / std::f32::consts::TAU + 1.0) % 1.0;
                    let base = hsv_to_rgb(hue, 0.7, 0.9);
                    let u_cell = (uv.x / self.checker_size).floor() as i32;
                    let v_cell = (uv.y / self.checker_size).floor() as i32;
                    let dim = if (u_cell + v_cell) % 2 == 0 { 1.0 } else { 0.6 };
                    let r = uv.length();
                    (base * dim * (1.0 - (-r * 2.0).exp() * 0.5)).extend(1.0)
                }
                ParamVizStyle::LocalRad => {
                    let angle = uv.y.atan2(uv.x);
                    let hue = (angle / std::f32::consts::TAU + 1.0) % 1.0;
                    let base = hsv_to_rgb(hue, 0.7, 0.9);
                    let r = uv.length();
                    let stripe =
                        f32::from(u8::from((r / self.checker_size).floor() as i32 % 2 == 0));
                    let dim = 0.6 + 0.4 * stripe;
                    (base * dim).extend(1.0)
                }
            })
            .collect()
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_parameterization_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.style,
            &mut self.checker_size,
            &mut self.checker_colors,
        )
    }
}

impl Quantity for MeshCornerParameterizationQuantity {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn structure_name(&self) -> &str {
        &self.structure_name
    }

    fn kind(&self) -> QuantityKind {
        QuantityKind::Parameterization
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
        self.coords.len()
    }
}

impl FaceQuantity for MeshCornerParameterizationQuantity {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_parameterization_creation() {
        let coords = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.5, 1.0),
        ];
        let q = MeshVertexParameterizationQuantity::new("uv", "mesh", coords);

        assert_eq!(q.name(), "uv");
        assert_eq!(q.structure_name(), "mesh");
        assert_eq!(q.data_size(), 3);
        assert_eq!(q.kind(), QuantityKind::Parameterization);
        assert!(!q.is_enabled());
        assert_eq!(q.style(), ParamVizStyle::Checker);
        assert_eq!(q.coords_type(), ParamCoordsType::Unit);
    }

    #[test]
    fn test_checker_colors_computation() {
        let coords = vec![
            Vec2::new(0.0, 0.0),   // cell (0,0) -> even -> color[0]
            Vec2::new(0.15, 0.0),  // cell (1,0) -> odd -> color[1]
            Vec2::new(0.0, 0.15),  // cell (0,1) -> odd -> color[1]
            Vec2::new(0.15, 0.15), // cell (1,1) -> even -> color[0]
        ];
        let q = MeshVertexParameterizationQuantity::new("uv", "mesh", coords);

        let colors = q.compute_colors();
        assert_eq!(colors.len(), 4);
        assert_eq!(colors[0], q.checker_colors()[0]);
        assert_eq!(colors[1], q.checker_colors()[1]);
        assert_eq!(colors[2], q.checker_colors()[1]);
        assert_eq!(colors[3], q.checker_colors()[0]);
    }

    #[test]
    fn test_grid_colors_computation() {
        let coords = vec![
            Vec2::new(0.001, 0.001), // near grid intersection -> on line
            Vec2::new(0.05, 0.05),   // center of cell -> not on line
        ];
        let mut q = MeshVertexParameterizationQuantity::new("uv", "mesh", coords);
        q.set_style(ParamVizStyle::Grid);

        let colors = q.compute_colors();
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0], q.checker_colors()[1]); // on line
        assert_eq!(colors[1], q.checker_colors()[0]); // off line
    }

    #[test]
    fn test_local_check_colors_computation() {
        let coords = vec![Vec2::new(0.5, 0.0), Vec2::new(0.0, 0.5)];
        let mut q = MeshVertexParameterizationQuantity::new("uv", "mesh", coords);
        q.set_style(ParamVizStyle::LocalCheck);

        let colors = q.compute_colors();
        assert_eq!(colors.len(), 2);
        // Colors should be non-zero (derived from HSV)
        assert!(colors[0].length() > 0.0);
        assert!(colors[1].length() > 0.0);
    }

    #[test]
    fn test_local_rad_colors_computation() {
        let coords = vec![Vec2::new(0.5, 0.0), Vec2::new(0.0, 0.5)];
        let mut q = MeshVertexParameterizationQuantity::new("uv", "mesh", coords);
        q.set_style(ParamVizStyle::LocalRad);

        let colors = q.compute_colors();
        assert_eq!(colors.len(), 2);
        assert!(colors[0].length() > 0.0);
        assert!(colors[1].length() > 0.0);
    }

    #[test]
    fn test_corner_parameterization_creation() {
        // 1 triangle = 3 corners
        let coords = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.5, 1.0),
        ];
        let q = MeshCornerParameterizationQuantity::new("uv_corners", "mesh", coords);

        assert_eq!(q.name(), "uv_corners");
        assert_eq!(q.data_size(), 3);
        assert_eq!(q.kind(), QuantityKind::Parameterization);
    }

    #[test]
    fn test_corner_parameterization_compute_colors() {
        let coords = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.15, 0.0),
            Vec2::new(0.0, 0.15),
        ];
        let q = MeshCornerParameterizationQuantity::new("uv_corners", "mesh", coords);

        let colors = q.compute_colors();
        assert_eq!(colors.len(), 3);
        assert_eq!(colors[0], q.checker_colors()[0]); // cell (0,0) even
        assert_eq!(colors[1], q.checker_colors()[1]); // cell (1,0) odd
        assert_eq!(colors[2], q.checker_colors()[1]); // cell (0,1) odd
    }

    #[test]
    fn test_hsv_to_rgb() {
        // Red
        let red = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((red.x - 1.0).abs() < 1e-5);
        assert!(red.y.abs() < 1e-5);
        assert!(red.z.abs() < 1e-5);

        // Green
        let green = hsv_to_rgb(1.0 / 3.0, 1.0, 1.0);
        assert!(green.x.abs() < 1e-5);
        assert!((green.y - 1.0).abs() < 1e-5);
        assert!(green.z.abs() < 1e-5);
    }
}
