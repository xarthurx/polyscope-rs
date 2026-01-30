//! Structure trait and related types.
//!
//! A [`Structure`] represents a geometric object in the scene, such as a point cloud,
//! surface mesh, or curve network.

use std::any::Any;

use glam::{Mat4, Vec3};

use crate::pick::PickResult;
use crate::quantity::Quantity;

/// A geometric object that can be visualized in polyscope.
///
/// Structures are the primary objects managed by polyscope. Each structure has:
/// - A unique name within its type
/// - A transform matrix for positioning in the scene
/// - Visibility state
/// - Methods for rendering and UI building
pub trait Structure: Any + Send + Sync {
    /// Returns a reference to self as `Any` for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Returns a mutable reference to self as `Any` for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Returns the unique name of this structure.
    fn name(&self) -> &str;

    /// Returns the type name of this structure (e.g., "`PointCloud`", "`SurfaceMesh`").
    fn type_name(&self) -> &'static str;

    /// Returns the axis-aligned bounding box in world coordinates.
    ///
    /// Returns `None` if the structure has no spatial extent.
    fn bounding_box(&self) -> Option<(Vec3, Vec3)>;

    /// Returns a characteristic length scale for this structure.
    fn length_scale(&self) -> f32;

    /// Returns the current model transform matrix.
    fn transform(&self) -> Mat4;

    /// Sets the model transform matrix.
    fn set_transform(&mut self, transform: Mat4);

    /// Returns whether this structure is currently visible.
    fn is_enabled(&self) -> bool;

    /// Sets the visibility of this structure.
    fn set_enabled(&mut self, enabled: bool);

    /// Draws this structure to the scene.
    ///
    /// Called during the main render pass.
    fn draw(&self, ctx: &mut dyn RenderContext);

    /// Draws this structure for picking/selection.
    ///
    /// Called during the pick render pass.
    fn draw_pick(&self, ctx: &mut dyn RenderContext);

    /// Builds the `ImGui` UI for this structure.
    fn build_ui(&mut self, ui: &dyn std::any::Any);

    /// Builds the `ImGui` UI for a picked element.
    fn build_pick_ui(&self, ui: &dyn std::any::Any, pick: &PickResult);

    /// Refreshes GPU resources after data changes.
    fn refresh(&mut self);

    /// Centers the camera on this structure's bounding box.
    fn center_bounding_box(&mut self) {
        // Default implementation - can be overridden
    }

    /// Resets the transform to identity.
    fn reset_transform(&mut self) {
        self.set_transform(Mat4::IDENTITY);
    }

    /// Returns the material name for this structure (e.g., "clay", "wax").
    fn material(&self) -> &str {
        "clay"
    }

    /// Sets the material for this structure by name.
    fn set_material(&mut self, _material: &str) {
        // Default no-op; structures that support materials override this
    }
}

/// A structure that can have quantities attached to it.
pub trait HasQuantities: Structure {
    /// Adds a quantity to this structure.
    fn add_quantity(&mut self, quantity: Box<dyn Quantity>);

    /// Gets a quantity by name.
    fn get_quantity(&self, name: &str) -> Option<&dyn Quantity>;

    /// Gets a mutable quantity by name.
    fn get_quantity_mut(&mut self, name: &str) -> Option<&mut Box<dyn Quantity>>;

    /// Removes a quantity by name.
    fn remove_quantity(&mut self, name: &str) -> Option<Box<dyn Quantity>>;

    /// Returns all quantities attached to this structure.
    fn quantities(&self) -> &[Box<dyn Quantity>];

    /// Returns the number of quantities attached.
    fn num_quantities(&self) -> usize {
        self.quantities().len()
    }
}

/// Trait for render context - will be implemented in polyscope-render.
///
/// This is a placeholder trait to avoid circular dependencies.
pub trait RenderContext: Send + Sync {}
