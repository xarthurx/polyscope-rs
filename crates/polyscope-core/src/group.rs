//! Group structure for organizing structures hierarchically.
//!
//! Groups allow organizing structures into a tree hierarchy. When a group is
//! disabled, all of its child structures and groups are also hidden.

use std::collections::HashSet;

/// A group that can contain structures and other groups.
///
/// Groups provide organizational hierarchy for structures in the viewer.
/// Enabling or disabling a group affects all of its descendants.
#[derive(Debug, Clone)]
pub struct Group {
    /// The unique name of this group.
    name: String,
    /// Whether this group is enabled (visible).
    enabled: bool,
    /// Whether to show child details in the UI.
    show_child_details: bool,
    /// Names of child structures (`type_name:name` format).
    child_structures: HashSet<String>,
    /// Names of child groups.
    child_groups: HashSet<String>,
    /// Name of parent group (if any).
    parent_group: Option<String>,
}

impl Group {
    /// Creates a new group with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            show_child_details: true,
            child_structures: HashSet::new(),
            child_groups: HashSet::new(),
            parent_group: None,
        }
    }

    /// Returns the name of this group.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether this group is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets whether this group is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns whether child details should be shown in UI.
    #[must_use]
    pub fn show_child_details(&self) -> bool {
        self.show_child_details
    }

    /// Sets whether child details should be shown in UI.
    pub fn set_show_child_details(&mut self, show: bool) {
        self.show_child_details = show;
    }

    /// Returns the parent group name, if any.
    #[must_use]
    pub fn parent_group(&self) -> Option<&str> {
        self.parent_group.as_deref()
    }

    /// Sets the parent group.
    pub fn set_parent_group(&mut self, parent: Option<String>) {
        self.parent_group = parent;
    }

    /// Adds a structure to this group.
    ///
    /// The structure is identified by "`type_name:name`" format.
    pub fn add_structure(&mut self, type_name: &str, name: &str) {
        self.child_structures.insert(format!("{type_name}:{name}"));
    }

    /// Removes a structure from this group.
    pub fn remove_structure(&mut self, type_name: &str, name: &str) {
        self.child_structures.remove(&format!("{type_name}:{name}"));
    }

    /// Returns whether this group contains a structure.
    #[must_use]
    pub fn contains_structure(&self, type_name: &str, name: &str) -> bool {
        self.child_structures
            .contains(&format!("{type_name}:{name}"))
    }

    /// Adds a child group.
    pub fn add_child_group(&mut self, group_name: &str) {
        self.child_groups.insert(group_name.to_string());
    }

    /// Removes a child group.
    pub fn remove_child_group(&mut self, group_name: &str) {
        self.child_groups.remove(group_name);
    }

    /// Returns whether this group contains a child group.
    #[must_use]
    pub fn contains_child_group(&self, group_name: &str) -> bool {
        self.child_groups.contains(group_name)
    }

    /// Returns the child structure identifiers.
    pub fn child_structures(&self) -> impl Iterator<Item = (&str, &str)> {
        self.child_structures.iter().filter_map(|s| {
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0], parts[1]))
            } else {
                None
            }
        })
    }

    /// Returns the child group names.
    pub fn child_groups(&self) -> impl Iterator<Item = &str> {
        self.child_groups.iter().map(std::string::String::as_str)
    }

    /// Returns the number of child structures.
    #[must_use]
    pub fn num_child_structures(&self) -> usize {
        self.child_structures.len()
    }

    /// Returns the number of child groups.
    #[must_use]
    pub fn num_child_groups(&self) -> usize {
        self.child_groups.len()
    }

    /// Returns true if this group has no children.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.child_structures.is_empty() && self.child_groups.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_creation() {
        let group = Group::new("test_group");
        assert_eq!(group.name(), "test_group");
        assert!(group.is_enabled());
        assert!(group.is_empty());
    }

    #[test]
    fn test_add_structure() {
        let mut group = Group::new("test");
        group.add_structure("PointCloud", "my_points");
        assert!(group.contains_structure("PointCloud", "my_points"));
        assert!(!group.is_empty());
        assert_eq!(group.num_child_structures(), 1);
    }

    #[test]
    fn test_remove_structure() {
        let mut group = Group::new("test");
        group.add_structure("PointCloud", "my_points");
        group.remove_structure("PointCloud", "my_points");
        assert!(!group.contains_structure("PointCloud", "my_points"));
        assert!(group.is_empty());
    }

    #[test]
    fn test_add_child_group() {
        let mut parent = Group::new("parent");
        parent.add_child_group("child");
        assert!(parent.contains_child_group("child"));
        assert_eq!(parent.num_child_groups(), 1);
    }

    #[test]
    fn test_parent_group() {
        let mut group = Group::new("child");
        assert!(group.parent_group().is_none());
        group.set_parent_group(Some("parent".to_string()));
        assert_eq!(group.parent_group(), Some("parent"));
    }

    #[test]
    fn test_enable_disable() {
        let mut group = Group::new("test");
        assert!(group.is_enabled());
        group.set_enabled(false);
        assert!(!group.is_enabled());
    }
}
