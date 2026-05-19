// Items are wired in by subsequent tasks (tet decomposition + face dispatch).
// The allow is removed at that point.
#![allow(dead_code)]

//! Static shape data for each `VolumeCellType`.
//!
//! Defines, for each cell type:
//! - the unique vertex indices on each face (used for canonical face hashing)
//! - the per-face triangulation stencil (used for rendering and edge detection)
//! - the tet-decomposition pattern (used for slicing and isosurface extraction)
//!
//! Index conventions mirror upstream C++ Polyscope (`src/volume_mesh.cpp`,
//! `stencilTet`, `stencilHex`, `stencilPrism`, `stencilPyramid`):
//! - Tet: 4 verts in slots 0..3, sentinels in 4..7. 4 triangular faces.
//! - Hex: 8 verts in 0..7. 6 quadrilateral faces, each split into 2 triangles.
//! - Prism: 6 verts in 0..5, sentinels in 6..7. Bottom tri (0,1,2), top tri (3,4,5).
//!   5 faces: 1 tri (bottom) + 3 quads (sides) + 1 tri (top) = 8 triangles.
//! - Pyramid: 5 verts in 0..4, sentinels in 5..7. Base quad (0,1,2,3), apex (4).
//!   5 faces: 1 quad (base) + 4 tris (sides) = 6 triangles.

use super::VolumeCellType;

/// A face is described by (unique vertex slot list, triangulation).
///
/// `polygon` lists the unique cell-local vertex slots in CCW order around the
/// face (3 for triangles, 4 for quads). `triangulation` is the list of triangle
/// stencils used to render the face.
pub struct FaceData {
    pub polygon: &'static [usize],
    pub triangulation: &'static [[usize; 3]],
}

// ===== Tet =====
const TET_FACES: &[FaceData] = &[
    FaceData { polygon: &[0, 2, 1], triangulation: &[[0, 2, 1]] },
    FaceData { polygon: &[0, 1, 3], triangulation: &[[0, 1, 3]] },
    FaceData { polygon: &[0, 3, 2], triangulation: &[[0, 3, 2]] },
    FaceData { polygon: &[1, 2, 3], triangulation: &[[1, 2, 3]] },
];

// ===== Hex =====
// Numbered like in the VTK file-formats diagram, with slots 6 and 7 swapped
// to match upstream (see polyscope/src/volume_mesh.cpp:43).
const HEX_FACES: &[FaceData] = &[
    FaceData { polygon: &[2, 1, 0, 3], triangulation: &[[2, 1, 0], [2, 0, 3]] }, // Bottom
    FaceData { polygon: &[4, 0, 1, 5], triangulation: &[[4, 0, 1], [4, 1, 5]] }, // Front
    FaceData { polygon: &[5, 1, 2, 6], triangulation: &[[5, 1, 2], [5, 2, 6]] }, // Right
    FaceData { polygon: &[7, 3, 0, 4], triangulation: &[[7, 3, 0], [7, 0, 4]] }, // Left
    FaceData { polygon: &[6, 2, 3, 7], triangulation: &[[6, 2, 3], [6, 3, 7]] }, // Back
    FaceData { polygon: &[7, 4, 5, 6], triangulation: &[[7, 4, 5], [7, 5, 6]] }, // Top
];

// ===== Prism (wedge) =====
// Slots 0,1,2 = bottom triangle; 3,4,5 = top triangle (slots 3,4,5 align with 0,1,2).
const PRISM_FACES: &[FaceData] = &[
    FaceData { polygon: &[0, 2, 1],    triangulation: &[[0, 2, 1]] },            // Bottom tri
    FaceData { polygon: &[0, 3, 5, 2], triangulation: &[[0, 5, 2], [0, 3, 5]] }, // Side quad 1
    FaceData { polygon: &[2, 5, 4, 1], triangulation: &[[2, 4, 5], [2, 1, 4]] }, // Side quad 2
    FaceData { polygon: &[0, 1, 4, 3], triangulation: &[[3, 0, 4], [0, 1, 4]] }, // Side quad 3
    FaceData { polygon: &[3, 4, 5],    triangulation: &[[3, 4, 5]] },            // Top tri
];

// ===== Pyramid =====
// Slots 0..3 = base quad (CCW from outside, looking from -apex toward base);
// Slot 4 = apex.
const PYRAMID_FACES: &[FaceData] = &[
    FaceData { polygon: &[0, 1, 2, 3], triangulation: &[[0, 3, 2], [0, 2, 1]] }, // Base quad
    FaceData { polygon: &[0, 1, 4],    triangulation: &[[0, 1, 4]] },            // Side 1
    FaceData { polygon: &[1, 2, 4],    triangulation: &[[1, 2, 4]] },            // Side 2
    FaceData { polygon: &[2, 3, 4],    triangulation: &[[2, 3, 4]] },            // Side 3
    FaceData { polygon: &[3, 0, 4],    triangulation: &[[3, 0, 4]] },            // Side 4
];

/// Returns the face data table for a given cell type.
#[must_use]
pub fn face_data_for(cell_type: VolumeCellType) -> &'static [FaceData] {
    match cell_type {
        VolumeCellType::Tet => TET_FACES,
        VolumeCellType::Hex => HEX_FACES,
        VolumeCellType::Prism => PRISM_FACES,
        VolumeCellType::Pyramid => PYRAMID_FACES,
    }
}

/// Number of vertices used by a cell type (non-sentinel slots in the `[u32; 8]`).
#[must_use]
pub fn num_verts_in_cell(cell_type: VolumeCellType) -> usize {
    match cell_type {
        VolumeCellType::Tet => 4,
        VolumeCellType::Hex => 8,
        VolumeCellType::Prism => 6,
        VolumeCellType::Pyramid => 5,
    }
}

/// Builds a canonical (sorted) face key for hashing.
///
/// Face polygons can have 3 or 4 unique vertices. Triangular faces leave slot 3
/// as `u32::MAX` so that triangle and quad keys never collide.
#[must_use]
pub fn canonical_face_key(cell: &[u32; 8], polygon: &[usize]) -> [u32; 4] {
    let mut key = [u32::MAX; 4];
    debug_assert!(polygon.len() == 3 || polygon.len() == 4);
    for (i, &slot) in polygon.iter().enumerate() {
        key[i] = cell[slot];
    }
    key.sort_unstable();
    key
}
