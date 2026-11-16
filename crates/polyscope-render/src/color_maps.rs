//! Color map system.

use glam::Vec3;

/// The built-in color map tables, in the order of C++ Polyscope.
///
/// Each table holds RGB triples of little-endian `f32` values. The data comes from
/// `src/render/color_maps.cpp` in C++ Polyscope (see `scripts/extract_colormaps.py`).
const BUILTIN_COLOR_MAP_TABLES: [(&str, &[u8]); 15] = [
    ("viridis", include_bytes!("../data/colormaps/viridis.bin")),
    ("plasma", include_bytes!("../data/colormaps/plasma.bin")),
    ("inferno", include_bytes!("../data/colormaps/inferno.bin")),
    ("magma", include_bytes!("../data/colormaps/magma.bin")),
    ("gray", include_bytes!("../data/colormaps/gray.bin")),
    ("coolwarm", include_bytes!("../data/colormaps/coolwarm.bin")),
    ("blues", include_bytes!("../data/colormaps/blues.bin")),
    ("reds", include_bytes!("../data/colormaps/reds.bin")),
    (
        "pink-green",
        include_bytes!("../data/colormaps/pink-green.bin"),
    ),
    ("phase", include_bytes!("../data/colormaps/phase.bin")),
    ("spectral", include_bytes!("../data/colormaps/spectral.bin")),
    ("rainbow", include_bytes!("../data/colormaps/rainbow.bin")),
    ("jet", include_bytes!("../data/colormaps/jet.bin")),
    ("turbo", include_bytes!("../data/colormaps/turbo.bin")),
    ("hsv", include_bytes!("../data/colormaps/hsv.bin")),
];

/// The names of the built-in color maps, in the order of C++ Polyscope.
///
/// Every [`ColorMapRegistry::new`] registry contains these maps. Color map
/// selectors in the UI offer this list.
pub const BUILTIN_COLOR_MAP_NAMES: [&str; 15] = {
    let mut names = [""; 15];
    let mut i = 0;
    while i < names.len() {
        names[i] = BUILTIN_COLOR_MAP_TABLES[i].0;
        i += 1;
    }
    names
};

/// Decodes a table of RGB triples of little-endian `f32` values.
fn decode_color_table(bytes: &[u8]) -> Vec<Vec3> {
    let (components, _) = bytes.as_chunks::<4>();
    let (colors, _) = components.as_chunks::<3>();
    colors
        .iter()
        .map(|rgb| Vec3::from_array(rgb.map(f32::from_le_bytes)))
        .collect()
}

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
    #[must_use]
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
///
/// The registry keeps the color maps in registration order. A new registry holds
/// the built-in maps in the order of [`BUILTIN_COLOR_MAP_NAMES`]. User maps follow.
#[derive(Default)]
pub struct ColorMapRegistry {
    color_maps: Vec<ColorMap>,
}

impl ColorMapRegistry {
    /// Creates a new color map registry with default color maps.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        for (name, bytes) in BUILTIN_COLOR_MAP_TABLES {
            self.register(ColorMap::new(name, decode_color_table(bytes)));
        }
    }

    /// Registers a color map.
    ///
    /// A map with a new name goes after all registered maps. A map with a known
    /// name replaces the old map at the same position.
    pub fn register(&mut self, color_map: ColorMap) {
        if let Some(existing) = self
            .color_maps
            .iter_mut()
            .find(|cm| cm.name == color_map.name)
        {
            *existing = color_map;
        } else {
            self.color_maps.push(color_map);
        }
    }

    /// Gets a color map by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ColorMap> {
        self.color_maps.iter().find(|cm| cm.name == name)
    }

    /// Returns all color map names in registration order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.color_maps.iter().map(|cm| cm.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: Vec3, expected: Vec3) {
        assert!(
            (actual - expected).abs().max_element() < 1e-6,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn test_every_builtin_name_resolves() {
        let registry = ColorMapRegistry::new();
        for name in BUILTIN_COLOR_MAP_NAMES {
            let color_map = registry.get(name).expect("built-in color map missing");
            assert_eq!(color_map.name, name);
            assert!(color_map.colors.len() >= 2, "{name} has too few colors");
        }
    }

    #[test]
    fn test_names_follow_cpp_order_then_user_maps() {
        let mut registry = ColorMapRegistry::new();
        let names: Vec<&str> = registry.names().collect();
        assert_eq!(names, BUILTIN_COLOR_MAP_NAMES);

        registry.register(ColorMap::new("user", vec![Vec3::ZERO, Vec3::ONE]));
        registry.register(ColorMap::new("blues", vec![Vec3::ZERO, Vec3::ONE]));
        let names: Vec<&str> = registry.names().collect();
        assert_eq!(names[..15], BUILTIN_COLOR_MAP_NAMES);
        assert_eq!(names[15..], ["user"]);
        assert_eq!(registry.get("blues").unwrap().colors.len(), 2);
    }

    #[test]
    fn test_viridis_matches_cpp_table() {
        let registry = ColorMapRegistry::new();
        let viridis = registry.get("viridis").unwrap();
        assert_close(
            viridis.colors[0],
            Vec3::new(0.267_004, 0.004_874, 0.329_415),
        );
        assert_close(
            *viridis.colors.last().unwrap(),
            Vec3::new(0.993_248, 0.906_157, 0.143_936),
        );
        assert_close(viridis.sample(0.0), viridis.colors[0]);
        assert_close(viridis.sample(1.0), *viridis.colors.last().unwrap());
    }

    #[test]
    fn test_sample_clamps_outside_unit_interval() {
        let registry = ColorMapRegistry::new();
        let viridis = registry.get("viridis").unwrap();
        assert_close(viridis.sample(-0.5), viridis.sample(0.0));
        assert_close(viridis.sample(1.5), viridis.sample(1.0));
    }
}
