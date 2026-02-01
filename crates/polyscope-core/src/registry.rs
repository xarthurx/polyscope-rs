//! Structure registry for managing registered structures.

use std::collections::HashMap;

use crate::error::{PolyscopeError, Result};
use crate::structure::Structure;

/// Registry for managing all structures in polyscope.
///
/// Structures are organized by type name and then by instance name.
#[derive(Default)]
pub struct Registry {
    /// Map from type name -> (instance name -> structure)
    structures: HashMap<String, HashMap<String, Box<dyn Structure>>>,
}

impl Registry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a structure with the registry.
    ///
    /// Returns an error if a structure with the same type and name already exists.
    pub fn register(&mut self, structure: Box<dyn Structure>) -> Result<()> {
        let type_name = structure.type_name().to_string();
        let name = structure.name().to_string();

        let type_map = self.structures.entry(type_name).or_default();

        if type_map.contains_key(&name) {
            return Err(PolyscopeError::StructureExists(name));
        }

        type_map.insert(name, structure);
        Ok(())
    }

    /// Gets a reference to a structure by type and name.
    #[must_use]
    pub fn get(&self, type_name: &str, name: &str) -> Option<&dyn Structure> {
        self.structures
            .get(type_name)
            .and_then(|m| m.get(name))
            .map(std::convert::AsRef::as_ref)
    }

    /// Gets a mutable reference to a structure by type and name.
    pub fn get_mut(&mut self, type_name: &str, name: &str) -> Option<&mut Box<dyn Structure>> {
        self.structures.get_mut(type_name)?.get_mut(name)
    }

    /// Checks if a structure with the given type and name exists.
    #[must_use]
    pub fn contains(&self, type_name: &str, name: &str) -> bool {
        self.structures
            .get(type_name)
            .is_some_and(|m| m.contains_key(name))
    }

    /// Removes a structure by type and name.
    pub fn remove(&mut self, type_name: &str, name: &str) -> Option<Box<dyn Structure>> {
        self.structures
            .get_mut(type_name)
            .and_then(|m| m.remove(name))
    }

    /// Removes all structures of a given type.
    pub fn remove_all_of_type(&mut self, type_name: &str) {
        self.structures.remove(type_name);
    }

    /// Removes all structures from the registry.
    pub fn clear(&mut self) {
        self.structures.clear();
    }

    /// Returns an iterator over all structures.
    pub fn iter(&self) -> impl Iterator<Item = &dyn Structure> {
        self.structures
            .values()
            .flat_map(|m| m.values())
            .map(std::convert::AsRef::as_ref)
    }

    /// Returns a mutable iterator over all structures.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn Structure>> + '_ {
        self.structures.values_mut().flat_map(|m| m.values_mut())
    }

    /// Returns the total number of registered structures.
    #[must_use]
    pub fn len(&self) -> usize {
        self.structures
            .values()
            .map(std::collections::HashMap::len)
            .sum()
    }

    /// Returns true if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.structures
            .values()
            .all(std::collections::HashMap::is_empty)
    }

    /// Returns all structures of a given type.
    pub fn get_all_of_type(&self, type_name: &str) -> impl Iterator<Item = &dyn Structure> {
        self.structures
            .get(type_name)
            .into_iter()
            .flat_map(|m| m.values())
            .map(std::convert::AsRef::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;

    use super::*;
    use crate::pick::PickResult;
    use glam::{Mat4, Vec3};

    /// Minimal mock structure for testing the registry.
    struct MockStructure {
        name: String,
        type_name: &'static str,
        enabled: bool,
        transform: Mat4,
    }

    impl MockStructure {
        fn new(name: &str, type_name: &'static str) -> Self {
            Self {
                name: name.to_string(),
                type_name,
                enabled: true,
                transform: Mat4::IDENTITY,
            }
        }
    }

    impl Structure for MockStructure {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn type_name(&self) -> &'static str {
            self.type_name
        }
        fn bounding_box(&self) -> Option<(Vec3, Vec3)> {
            None
        }
        fn length_scale(&self) -> f32 {
            1.0
        }
        fn transform(&self) -> Mat4 {
            self.transform
        }
        fn set_transform(&mut self, transform: Mat4) {
            self.transform = transform;
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }
        fn draw(&self, _ctx: &mut dyn crate::structure::RenderContext) {}
        fn draw_pick(&self, _ctx: &mut dyn crate::structure::RenderContext) {}
        fn build_ui(&mut self, _ui: &dyn Any) {}
        fn build_pick_ui(&self, _ui: &dyn Any, _pick: &PickResult) {}
        fn refresh(&mut self) {}
    }

    fn mock(name: &str, type_name: &'static str) -> Box<dyn Structure> {
        Box::new(MockStructure::new(name, type_name))
    }

    #[test]
    fn test_new_registry_is_empty() {
        let reg = Registry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = Registry::new();
        reg.register(mock("bunny", "SurfaceMesh")).unwrap();

        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
        assert!(reg.contains("SurfaceMesh", "bunny"));

        let s = reg.get("SurfaceMesh", "bunny").unwrap();
        assert_eq!(s.name(), "bunny");
        assert_eq!(s.type_name(), "SurfaceMesh");
    }

    #[test]
    fn test_register_duplicate_errors() {
        let mut reg = Registry::new();
        reg.register(mock("bunny", "SurfaceMesh")).unwrap();

        let result = reg.register(mock("bunny", "SurfaceMesh"));
        assert!(result.is_err());
    }

    #[test]
    fn test_same_name_different_types() {
        let mut reg = Registry::new();
        reg.register(mock("data", "SurfaceMesh")).unwrap();
        reg.register(mock("data", "PointCloud")).unwrap();

        assert_eq!(reg.len(), 2);
        assert!(reg.contains("SurfaceMesh", "data"));
        assert!(reg.contains("PointCloud", "data"));
    }

    #[test]
    fn test_get_nonexistent() {
        let reg = Registry::new();
        assert!(reg.get("SurfaceMesh", "bunny").is_none());
        assert!(!reg.contains("SurfaceMesh", "bunny"));
    }

    #[test]
    fn test_get_mut() {
        let mut reg = Registry::new();
        reg.register(mock("bunny", "SurfaceMesh")).unwrap();

        let s = reg.get_mut("SurfaceMesh", "bunny").unwrap();
        s.set_enabled(false);

        let s = reg.get("SurfaceMesh", "bunny").unwrap();
        assert!(!s.is_enabled());
    }

    #[test]
    fn test_remove() {
        let mut reg = Registry::new();
        reg.register(mock("bunny", "SurfaceMesh")).unwrap();

        let removed = reg.remove("SurfaceMesh", "bunny");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name(), "bunny");
        assert!(reg.get("SurfaceMesh", "bunny").is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut reg = Registry::new();
        assert!(reg.remove("SurfaceMesh", "bunny").is_none());
    }

    #[test]
    fn test_remove_all_of_type() {
        let mut reg = Registry::new();
        reg.register(mock("a", "SurfaceMesh")).unwrap();
        reg.register(mock("b", "SurfaceMesh")).unwrap();
        reg.register(mock("c", "PointCloud")).unwrap();

        reg.remove_all_of_type("SurfaceMesh");
        assert_eq!(reg.len(), 1);
        assert!(!reg.contains("SurfaceMesh", "a"));
        assert!(reg.contains("PointCloud", "c"));
    }

    #[test]
    fn test_clear() {
        let mut reg = Registry::new();
        reg.register(mock("a", "SurfaceMesh")).unwrap();
        reg.register(mock("b", "PointCloud")).unwrap();

        reg.clear();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_iter() {
        let mut reg = Registry::new();
        reg.register(mock("a", "SurfaceMesh")).unwrap();
        reg.register(mock("b", "PointCloud")).unwrap();
        reg.register(mock("c", "SurfaceMesh")).unwrap();

        let names: Vec<&str> = reg.iter().map(|s| s.name()).collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
    }

    #[test]
    fn test_get_all_of_type() {
        let mut reg = Registry::new();
        reg.register(mock("a", "SurfaceMesh")).unwrap();
        reg.register(mock("b", "SurfaceMesh")).unwrap();
        reg.register(mock("c", "PointCloud")).unwrap();

        let meshes: Vec<&str> = reg.get_all_of_type("SurfaceMesh").map(|s| s.name()).collect();
        assert_eq!(meshes.len(), 2);
        assert!(meshes.contains(&"a"));
        assert!(meshes.contains(&"b"));
    }

    #[test]
    fn test_get_all_of_type_empty() {
        let reg = Registry::new();
        assert_eq!(reg.get_all_of_type("SurfaceMesh").count(), 0);
    }
}
