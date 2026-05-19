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
    FaceData {
        polygon: &[0, 2, 1],
        triangulation: &[[0, 2, 1]],
    },
    FaceData {
        polygon: &[0, 1, 3],
        triangulation: &[[0, 1, 3]],
    },
    FaceData {
        polygon: &[0, 3, 2],
        triangulation: &[[0, 3, 2]],
    },
    FaceData {
        polygon: &[1, 2, 3],
        triangulation: &[[1, 2, 3]],
    },
];

// ===== Hex =====
// Numbered like in the VTK file-formats diagram, with slots 6 and 7 swapped
// to match upstream (see polyscope/src/volume_mesh.cpp:43).
const HEX_FACES: &[FaceData] = &[
    FaceData {
        polygon: &[2, 1, 0, 3],
        triangulation: &[[2, 1, 0], [2, 0, 3]],
    }, // Bottom
    FaceData {
        polygon: &[4, 0, 1, 5],
        triangulation: &[[4, 0, 1], [4, 1, 5]],
    }, // Front
    FaceData {
        polygon: &[5, 1, 2, 6],
        triangulation: &[[5, 1, 2], [5, 2, 6]],
    }, // Right
    FaceData {
        polygon: &[7, 3, 0, 4],
        triangulation: &[[7, 3, 0], [7, 0, 4]],
    }, // Left
    FaceData {
        polygon: &[6, 2, 3, 7],
        triangulation: &[[6, 2, 3], [6, 3, 7]],
    }, // Back
    FaceData {
        polygon: &[7, 4, 5, 6],
        triangulation: &[[7, 4, 5], [7, 5, 6]],
    }, // Top
];

// ===== Prism (wedge) =====
// Slots 0,1,2 = bottom triangle; 3,4,5 = top triangle (slots 3,4,5 align with 0,1,2).
const PRISM_FACES: &[FaceData] = &[
    FaceData {
        polygon: &[0, 2, 1],
        triangulation: &[[0, 2, 1]],
    }, // Bottom tri
    FaceData {
        polygon: &[0, 3, 5, 2],
        triangulation: &[[0, 5, 2], [0, 3, 5]],
    }, // Side quad 1
    FaceData {
        polygon: &[2, 5, 4, 1],
        triangulation: &[[2, 5, 4], [4, 1, 2]],
    }, // Side quad 2 (winding fixed vs. upstream stencilPrism, which is inward here)
    FaceData {
        polygon: &[0, 1, 4, 3],
        triangulation: &[[3, 0, 4], [0, 1, 4]],
    }, // Side quad 3
    FaceData {
        polygon: &[3, 4, 5],
        triangulation: &[[3, 4, 5]],
    }, // Top tri
];

// ===== Pyramid =====
// Slots 0..3 = base quad (CCW from outside, looking from -apex toward base);
// Slot 4 = apex.
const PYRAMID_FACES: &[FaceData] = &[
    FaceData {
        polygon: &[0, 1, 2, 3],
        triangulation: &[[0, 3, 2], [0, 2, 1]],
    }, // Base quad
    FaceData {
        polygon: &[0, 1, 4],
        triangulation: &[[0, 1, 4]],
    }, // Side 1
    FaceData {
        polygon: &[1, 2, 4],
        triangulation: &[[1, 2, 4]],
    }, // Side 2
    FaceData {
        polygon: &[2, 3, 4],
        triangulation: &[[2, 3, 4]],
    }, // Side 3
    FaceData {
        polygon: &[3, 0, 4],
        triangulation: &[[3, 0, 4]],
    }, // Side 4
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

/// Classifies a cell by examining its trailing sentinel slots.
///
/// Sentinels are always placed at the end of the 8-slot array, so the cell
/// type is fully determined by which of slots 4/5/6 are sentinel: this
/// requires at most 3 comparisons (Tet hits the first).
#[must_use]
pub fn cell_type_of(cell: &[u32; 8]) -> VolumeCellType {
    if cell[4] == u32::MAX {
        VolumeCellType::Tet // 4 sentinels in slots 4..7
    } else if cell[5] == u32::MAX {
        VolumeCellType::Pyramid // 3 sentinels in slots 5..7
    } else if cell[6] == u32::MAX {
        VolumeCellType::Prism // 2 sentinels in slots 6..7
    } else {
        VolumeCellType::Hex // 0 sentinels
    }
}

/// Number of real (non-sentinel) vertices in each cell type.
#[must_use]
pub fn num_real_verts(cell_type: VolumeCellType) -> usize {
    match cell_type {
        VolumeCellType::Tet => 4,
        VolumeCellType::Pyramid => 5,
        VolumeCellType::Prism => 6,
        VolumeCellType::Hex => 8,
    }
}

/// Number of tetrahedra produced by `decompose_cell_to_tets` for each cell type.
#[must_use]
pub fn num_tets_in_cell(cell_type: VolumeCellType) -> usize {
    match cell_type {
        VolumeCellType::Tet => 1,
        VolumeCellType::Pyramid => 2,
        VolumeCellType::Prism => 3,
        VolumeCellType::Hex => 5,
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

/// Decomposes a hex cell into 5 tetrahedra using a fixed diagonal pattern.
///
/// Central-diagonal pattern from Dompierre et al.; the 5-tet split works for
/// any convex hex. Matches the previous polyscope-rs decomposition.
pub(super) const HEX_TO_TET_PATTERN: [[usize; 4]; 5] = [
    [0, 1, 2, 5],
    [0, 2, 7, 5],
    [0, 2, 3, 7],
    [0, 5, 7, 4],
    [2, 7, 5, 6],
];

/// Decomposes a triangular prism into 3 tetrahedra.
///
/// Picks a consistent diagonal split on the quad face opposite the
/// lowest-numbered vertex, matching the upstream algorithm (`decomposePrism` in
/// `polyscope/src/volume_mesh.cpp`). Consistency across adjacent cells matters
/// so that shared faces are tessellated identically and no gaps appear in
/// slice caps for mixed meshes.
#[must_use]
pub fn decompose_prism(cell: &[u32; 8]) -> [[u32; 4]; 3] {
    let mut p: [u32; 6] = [cell[0], cell[1], cell[2], cell[3], cell[4], cell[5]];

    let min_idx = (0..6).min_by_key(|&i| p[i]).unwrap();

    if min_idx < 3 {
        let rot = match min_idx {
            0 => 0,
            1 => 2,
            _ => 1,
        };
        rotate_prism_in_place(&mut p, rot);
    } else {
        let top_pos = min_idx - 3;
        let rot = match top_pos {
            0 => 0,
            1 => 2,
            _ => 1,
        };
        p.swap(0, 3);
        p.swap(1, 4);
        p.swap(2, 5);
        rotate_prism_in_place(&mut p, rot);
    }

    if p[2].min(p[4]) < p[1].min(p[5]) {
        [
            [p[0], p[5], p[4], p[3]],
            [p[0], p[4], p[5], p[2]],
            [p[0], p[4], p[2], p[1]],
        ]
    } else {
        [
            [p[0], p[5], p[4], p[3]],
            [p[0], p[1], p[5], p[2]],
            [p[0], p[5], p[1], p[4]],
        ]
    }
}

fn rotate_prism_in_place(p: &mut [u32; 6], rot: usize) {
    const BOTTOM_ROT: [[usize; 3]; 3] = [[0, 1, 2], [1, 2, 0], [2, 0, 1]];
    const TOP_ROT: [[usize; 3]; 3] = [[3, 4, 5], [4, 5, 3], [5, 3, 4]];
    let src = *p;
    for i in 0..3 {
        p[i] = src[BOTTOM_ROT[rot][i]];
        p[i + 3] = src[TOP_ROT[rot][i]];
    }
}

/// Decomposes a square pyramid into 2 tetrahedra by splitting the base quad
/// along the diagonal containing the smaller of `{p[0], p[2]}` vs `{p[1], p[3]}`.
/// Consistent split ensures adjacent cells tessellate the shared face the same way.
#[must_use]
pub fn decompose_pyramid(cell: &[u32; 8]) -> [[u32; 4]; 2] {
    let p: [u32; 5] = [cell[0], cell[1], cell[2], cell[3], cell[4]];

    if p[0].min(p[2]) < p[1].min(p[3]) {
        [[p[0], p[2], p[4], p[1]], [p[0], p[4], p[2], p[3]]]
    } else {
        [[p[1], p[3], p[4], p[2]], [p[1], p[4], p[3], p[0]]]
    }
}

/// Invokes `f` once per tet in the decomposition of `cell`, dispatching on
/// cell type. Avoids the per-cell `Vec` allocation of returning a collection.
pub fn for_each_tet<F: FnMut([u32; 4])>(cell: &[u32; 8], cell_type: VolumeCellType, mut f: F) {
    match cell_type {
        VolumeCellType::Tet => f([cell[0], cell[1], cell[2], cell[3]]),
        VolumeCellType::Hex => {
            for t in &HEX_TO_TET_PATTERN {
                f([cell[t[0]], cell[t[1]], cell[t[2]], cell[t[3]]]);
            }
        }
        VolumeCellType::Prism => {
            for tet in decompose_prism(cell) {
                f(tet);
            }
        }
        VolumeCellType::Pyramid => {
            for tet in decompose_pyramid(cell) {
                f(tet);
            }
        }
    }
}
