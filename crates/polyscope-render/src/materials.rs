//! Material system.

use std::collections::HashMap;

/// A material definition for rendering.
#[derive(Debug, Clone)]
pub struct Material {
    /// Material name.
    pub name: String,
    /// Whether this is a flat (unlit) material.
    pub is_flat: bool,
    // TODO: Add material properties (textures, colors, etc.)
}

impl Material {
    /// Creates a new material.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_flat: false,
        }
    }

    /// Creates a flat (unlit) material.
    pub fn flat(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_flat: true,
        }
    }
}

/// Registry for managing materials.
#[derive(Default)]
pub struct MaterialRegistry {
    materials: HashMap<String, Material>,
}

impl MaterialRegistry {
    /// Creates a new material registry with default materials.
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        // Register default materials (matching C++ Polyscope)
        self.register(Material::new("clay"));
        self.register(Material::new("wax"));
        self.register(Material::new("candy"));
        self.register(Material::new("ceramic"));
        self.register(Material::new("jade"));
        self.register(Material::new("mud"));
        self.register(Material::new("normal"));
        self.register(Material::flat("flat"));
    }

    /// Registers a material.
    pub fn register(&mut self, material: Material) {
        self.materials.insert(material.name.clone(), material);
    }

    /// Gets a material by name.
    pub fn get(&self, name: &str) -> Option<&Material> {
        self.materials.get(name)
    }

    /// Returns all material names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.materials.keys().map(|s| s.as_str())
    }
}
