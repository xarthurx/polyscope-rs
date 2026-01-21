//! Color map system.

use std::collections::HashMap;

use glam::Vec3;

/// A color map for mapping scalar values to colors.
#[derive(Debug, Clone)]
pub struct ColorMap {
    /// Color map name.
    pub name: String,
    /// Color samples (evenly spaced from 0 to 1).
    pub colors: Vec<Vec3>,
}

impl ColorMap {
    /// Creates a new color map.
    pub fn new(name: impl Into<String>, colors: Vec<Vec3>) -> Self {
        Self {
            name: name.into(),
            colors,
        }
    }

    /// Samples the color map at a given value (0 to 1).
    pub fn sample(&self, t: f32) -> Vec3 {
        let t = t.clamp(0.0, 1.0);

        if self.colors.is_empty() {
            return Vec3::ZERO;
        }

        if self.colors.len() == 1 {
            return self.colors[0];
        }

        let n = self.colors.len() - 1;
        let idx = (t * n as f32).floor() as usize;
        let idx = idx.min(n - 1);
        let frac = t * n as f32 - idx as f32;

        self.colors[idx].lerp(self.colors[idx + 1], frac)
    }
}

/// Registry for managing color maps.
#[derive(Default)]
pub struct ColorMapRegistry {
    color_maps: HashMap<String, ColorMap>,
}

impl ColorMapRegistry {
    /// Creates a new color map registry with default color maps.
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        // Viridis color map
        self.register(ColorMap::new(
            "viridis",
            vec![
                Vec3::new(0.267, 0.004, 0.329),
                Vec3::new(0.282, 0.140, 0.457),
                Vec3::new(0.253, 0.265, 0.529),
                Vec3::new(0.206, 0.371, 0.553),
                Vec3::new(0.163, 0.471, 0.558),
                Vec3::new(0.127, 0.566, 0.550),
                Vec3::new(0.134, 0.658, 0.517),
                Vec3::new(0.266, 0.749, 0.440),
                Vec3::new(0.477, 0.821, 0.318),
                Vec3::new(0.741, 0.873, 0.150),
                Vec3::new(0.993, 0.906, 0.144),
            ],
        ));

        // Blues color map
        self.register(ColorMap::new(
            "blues",
            vec![
                Vec3::new(0.969, 0.984, 1.000),
                Vec3::new(0.871, 0.922, 0.969),
                Vec3::new(0.776, 0.859, 0.937),
                Vec3::new(0.620, 0.792, 0.882),
                Vec3::new(0.419, 0.682, 0.839),
                Vec3::new(0.259, 0.573, 0.776),
                Vec3::new(0.129, 0.443, 0.710),
                Vec3::new(0.031, 0.318, 0.612),
                Vec3::new(0.031, 0.188, 0.420),
            ],
        ));

        // Reds color map
        self.register(ColorMap::new(
            "reds",
            vec![
                Vec3::new(1.000, 0.961, 0.941),
                Vec3::new(0.996, 0.878, 0.824),
                Vec3::new(0.988, 0.733, 0.631),
                Vec3::new(0.988, 0.573, 0.447),
                Vec3::new(0.984, 0.416, 0.290),
                Vec3::new(0.937, 0.231, 0.173),
                Vec3::new(0.796, 0.094, 0.114),
                Vec3::new(0.647, 0.059, 0.082),
                Vec3::new(0.404, 0.000, 0.051),
            ],
        ));

        // Coolwarm color map
        self.register(ColorMap::new(
            "coolwarm",
            vec![
                Vec3::new(0.230, 0.299, 0.754),
                Vec3::new(0.552, 0.690, 0.996),
                Vec3::new(0.866, 0.866, 0.866),
                Vec3::new(0.956, 0.604, 0.486),
                Vec3::new(0.706, 0.016, 0.150),
            ],
        ));

        // Rainbow color map
        self.register(ColorMap::new(
            "rainbow",
            vec![
                Vec3::new(0.5, 0.0, 1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.0, 1.0, 1.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
            ],
        ));
    }

    /// Registers a color map.
    pub fn register(&mut self, color_map: ColorMap) {
        self.color_maps.insert(color_map.name.clone(), color_map);
    }

    /// Gets a color map by name.
    pub fn get(&self, name: &str) -> Option<&ColorMap> {
        self.color_maps.get(name)
    }

    /// Returns all color map names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.color_maps.keys().map(|s| s.as_str())
    }
}
