//! Error types for polyscope-rs.

use thiserror::Error;

/// The main error type for polyscope-rs operations.
#[derive(Error, Debug)]
pub enum PolyscopeError {
    /// Polyscope has not been initialized.
    #[error("polyscope not initialized - call polyscope_rs::init() first")]
    NotInitialized,

    /// Polyscope has already been initialized.
    #[error("polyscope already initialized")]
    AlreadyInitialized,

    /// A structure with the given name already exists.
    #[error("structure '{0}' already exists")]
    StructureExists(String),

    /// A structure with the given name was not found.
    #[error("structure '{0}' not found")]
    StructureNotFound(String),

    /// A quantity with the given name already exists.
    #[error("quantity '{0}' already exists on structure '{1}'")]
    QuantityExists(String, String),

    /// A quantity with the given name was not found.
    #[error("quantity '{0}' not found on structure '{1}'")]
    QuantityNotFound(String, String),

    /// A material with the given name already exists.
    #[error("material '{0}' already exists")]
    MaterialExists(String),

    /// Failed to load a material image.
    #[error("material load error: {0}")]
    MaterialLoadError(String),

    /// Data size mismatch.
    #[error("data size mismatch: expected {expected}, got {actual}")]
    SizeMismatch { expected: usize, actual: usize },

    /// Rendering error.
    #[error("render error: {0}")]
    RenderError(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// A specialized Result type for polyscope-rs operations.
pub type Result<T> = std::result::Result<T, PolyscopeError>;
