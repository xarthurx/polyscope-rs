//! Material system for surface rendering.
//!
//! Materials define how surfaces are shaded, including lighting properties
//! like ambient, diffuse, and specular reflection.

use std::collections::HashMap;

/// A material definition for rendering.
///
/// Materials control the appearance of surfaces through lighting parameters.
/// The default materials are designed to match the visual style of C++ Polyscope.
#[derive(Debug, Clone)]
pub struct Material {
    /// Material name.
    pub name: String,
    /// Whether this is a flat (unlit) material.
    pub is_flat: bool,
    /// Ambient light factor (0.0 - 1.0).
    pub ambient: f32,
    /// Diffuse reflection factor (0.0 - 1.0).
    pub diffuse: f32,
    /// Specular reflection intensity (0.0 - 1.0).
    pub specular: f32,
    /// Specular shininess/exponent (higher = sharper highlights).
    pub shininess: f32,
}

impl Material {
    /// Creates a new material with default properties.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_flat: false,
            ambient: 0.2,
            diffuse: 0.7,
            specular: 0.3,
            shininess: 32.0,
        }
    }

    /// Creates a new material with custom properties.
    pub fn with_properties(
        name: impl Into<String>,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
    ) -> Self {
        Self {
            name: name.into(),
            is_flat: false,
            ambient,
            diffuse,
            specular,
            shininess,
        }
    }

    /// Creates a flat (unlit) material.
    pub fn flat(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_flat: true,
            ambient: 1.0,
            diffuse: 0.0,
            specular: 0.0,
            shininess: 1.0,
        }
    }

    /// Creates the "clay" material - matte, minimal specularity.
    #[must_use] 
    pub fn clay() -> Self {
        Self::with_properties("clay", 0.25, 0.75, 0.1, 8.0)
    }

    /// Creates the "wax" material - slightly glossy, soft highlights.
    #[must_use] 
    pub fn wax() -> Self {
        Self::with_properties("wax", 0.2, 0.7, 0.4, 16.0)
    }

    /// Creates the "candy" material - shiny, bright highlights.
    #[must_use] 
    pub fn candy() -> Self {
        Self::with_properties("candy", 0.15, 0.6, 0.7, 64.0)
    }

    /// Creates the "ceramic" material - smooth, moderate gloss.
    #[must_use] 
    pub fn ceramic() -> Self {
        Self::with_properties("ceramic", 0.2, 0.65, 0.5, 32.0)
    }

    /// Creates the "jade" material - translucent appearance (simulated).
    #[must_use] 
    pub fn jade() -> Self {
        Self::with_properties("jade", 0.3, 0.6, 0.3, 24.0)
    }

    /// Creates the "mud" material - very matte, no specularity.
    #[must_use] 
    pub fn mud() -> Self {
        Self::with_properties("mud", 0.3, 0.7, 0.0, 1.0)
    }

    /// Creates the "normal" material - balanced properties.
    #[must_use] 
    pub fn normal() -> Self {
        Self::with_properties("normal", 0.2, 0.7, 0.3, 32.0)
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::clay()
    }
}

/// GPU-compatible material uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniforms {
    /// Ambient factor.
    pub ambient: f32,
    /// Diffuse factor.
    pub diffuse: f32,
    /// Specular intensity.
    pub specular: f32,
    /// Shininess exponent.
    pub shininess: f32,
}

impl From<&Material> for MaterialUniforms {
    fn from(mat: &Material) -> Self {
        Self {
            ambient: mat.ambient,
            diffuse: mat.diffuse,
            specular: mat.specular,
            shininess: mat.shininess,
        }
    }
}

impl Default for MaterialUniforms {
    fn default() -> Self {
        Self {
            ambient: 0.2,
            diffuse: 0.7,
            specular: 0.3,
            shininess: 32.0,
        }
    }
}

/// Registry for managing materials.
#[derive(Default)]
pub struct MaterialRegistry {
    materials: HashMap<String, Material>,
    default_material: String,
}

impl MaterialRegistry {
    /// Creates a new material registry with default materials.
    #[must_use] 
    pub fn new() -> Self {
        let mut registry = Self {
            materials: HashMap::new(),
            default_material: "clay".to_string(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        // Register default materials (matching C++ Polyscope style)
        self.register(Material::clay());
        self.register(Material::wax());
        self.register(Material::candy());
        self.register(Material::ceramic());
        self.register(Material::jade());
        self.register(Material::mud());
        self.register(Material::normal());
        self.register(Material::flat("flat"));
    }

    /// Registers a material.
    pub fn register(&mut self, material: Material) {
        self.materials.insert(material.name.clone(), material);
    }

    /// Gets a material by name.
    #[must_use] 
    pub fn get(&self, name: &str) -> Option<&Material> {
        self.materials.get(name)
    }

    /// Gets the default material.
    #[must_use] 
    pub fn default_material(&self) -> &Material {
        self.materials
            .get(&self.default_material)
            .unwrap_or_else(|| {
                self.materials
                    .values()
                    .next()
                    .expect("no materials registered")
            })
    }

    /// Sets the default material name.
    pub fn set_default(&mut self, name: &str) {
        if self.materials.contains_key(name) {
            self.default_material = name.to_string();
        }
    }

    /// Returns all material names.
    #[must_use] 
    pub fn names(&self) -> Vec<&str> {
        self.materials.keys().map(std::string::String::as_str).collect()
    }

    /// Returns the number of registered materials.
    #[must_use] 
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Returns true if no materials are registered.
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_creation() {
        let mat = Material::clay();
        assert_eq!(mat.name, "clay");
        assert!(!mat.is_flat);
        assert!(mat.ambient > 0.0);
    }

    #[test]
    fn test_flat_material() {
        let mat = Material::flat("test_flat");
        assert!(mat.is_flat);
        assert_eq!(mat.diffuse, 0.0);
        assert_eq!(mat.specular, 0.0);
    }

    #[test]
    fn test_material_registry() {
        let registry = MaterialRegistry::new();
        assert!(registry.get("clay").is_some());
        assert!(registry.get("wax").is_some());
        assert!(registry.get("candy").is_some());
        assert!(registry.get("flat").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_material_uniforms() {
        let mat = Material::candy();
        let uniforms = MaterialUniforms::from(&mat);
        assert_eq!(uniforms.ambient, mat.ambient);
        assert_eq!(uniforms.specular, mat.specular);
    }

    #[test]
    fn test_default_material() {
        let registry = MaterialRegistry::new();
        let default = registry.default_material();
        assert_eq!(default.name, "clay");
    }
}
