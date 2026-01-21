//! Curve network structure.

// TODO: Implement CurveNetwork
// This will include:
// - Nodes (points) and edges
// - Ribbon and cylinder rendering
// - Node and edge quantities

/// A curve network structure (nodes connected by edges).
pub struct CurveNetwork {
    name: String,
    // TODO: Add nodes, edges, quantities, GPU resources
}

impl CurveNetwork {
    /// Creates a new curve network (placeholder).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }
}
