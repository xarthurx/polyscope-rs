//! Floating quantities — screen-space data not attached to any structure.
//!
//! Floating quantities include images (scalar, color, depth-composited)
//! that are displayed in the UI as standalone visualizations.

mod scalar_image;
mod color_image;

pub use scalar_image::*;
pub use color_image::*;

/// Image origin convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageOrigin {
    /// Image row 0 is the top row (standard screen convention).
    #[default]
    UpperLeft,
    /// Image row 0 is the bottom row (OpenGL convention).
    LowerLeft,
}
