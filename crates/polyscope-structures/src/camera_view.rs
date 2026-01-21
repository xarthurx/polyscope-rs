//! Camera view structure.

// TODO: Implement CameraView
// This will include:
// - Camera frustum visualization
// - Camera parameters (intrinsics, extrinsics)
// - Image plane display

/// A camera view structure for visualizing camera poses.
pub struct CameraView {
    name: String,
    // TODO: Add camera parameters, GPU resources
}

impl CameraView {
    /// Creates a new camera view (placeholder).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }
}
