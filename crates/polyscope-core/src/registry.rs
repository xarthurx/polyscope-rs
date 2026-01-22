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
    pub fn get(&self, type_name: &str, name: &str) -> Option<&dyn Structure> {
        self.structures
            .get(type_name)
            .and_then(|m| m.get(name))
            .map(|s| s.as_ref())
    }

    /// Gets a mutable reference to a structure by type and name.
    pub fn get_mut(&mut self, type_name: &str, name: &str) -> Option<&mut Box<dyn Structure>> {
        self.structures.get_mut(type_name)?.get_mut(name)
    }

    /// Checks if a structure with the given type and name exists.
    pub fn contains(&self, type_name: &str, name: &str) -> bool {
        self.structures
            .get(type_name)
            .map_or(false, |m| m.contains_key(name))
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
            .map(|s| s.as_ref())
    }

    /// Returns a mutable iterator over all structures.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn Structure>> + '_ {
        self.structures.values_mut().flat_map(|m| m.values_mut())
    }

    /// Returns the total number of registered structures.
    pub fn len(&self) -> usize {
        self.structures.values().map(|m| m.len()).sum()
    }

    /// Returns true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.structures.values().all(|m| m.is_empty())
    }

    /// Returns all structures of a given type.
    pub fn get_all_of_type(&self, type_name: &str) -> impl Iterator<Item = &dyn Structure> {
        self.structures
            .get(type_name)
            .into_iter()
            .flat_map(|m| m.values())
            .map(|s| s.as_ref())
    }
}

#[cfg(test)]
mod tests {
    // TODO: Add tests once we have concrete structure implementations
}
