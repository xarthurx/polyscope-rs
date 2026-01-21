//! Quantity trait and related types.
//!
//! A [`Quantity`] represents data associated with a structure, such as scalar values,
//! vector fields, or colors.

/// The kind of quantity (for categorization and UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuantityKind {
    /// Scalar values (single float per element).
    Scalar,
    /// Vector values (Vec3 per element).
    Vector,
    /// Color values (RGB or RGBA per element).
    Color,
    /// Parameterization values (UV coordinates).
    Parameterization,
    /// Other/custom quantity type.
    Other,
}

/// Data associated with a structure that can be visualized.
///
/// Quantities are attached to structures and represent data like:
/// - Scalar fields (temperature, pressure, etc.)
/// - Vector fields (velocity, normals, etc.)
/// - Colors
/// - UV parameterizations
pub trait Quantity: Send + Sync {
    /// Returns the name of this quantity.
    fn name(&self) -> &str;

    /// Returns the name of the parent structure.
    fn structure_name(&self) -> &str;

    /// Returns the kind of this quantity.
    fn kind(&self) -> QuantityKind;

    /// Returns whether this quantity is currently enabled/visible.
    fn is_enabled(&self) -> bool;

    /// Sets the enabled state of this quantity.
    fn set_enabled(&mut self, enabled: bool);

    /// Builds the ImGui UI controls for this quantity.
    fn build_ui(&mut self, ui: &dyn std::any::Any);

    /// Refreshes GPU resources after data changes.
    fn refresh(&mut self);

    /// Returns the number of data elements.
    fn data_size(&self) -> usize;
}

/// Marker trait for quantities defined on vertices.
pub trait VertexQuantity: Quantity {}

/// Marker trait for quantities defined on faces.
pub trait FaceQuantity: Quantity {}

/// Marker trait for quantities defined on edges.
pub trait EdgeQuantity: Quantity {}

/// Marker trait for quantities defined on cells (for volume meshes).
pub trait CellQuantity: Quantity {}
