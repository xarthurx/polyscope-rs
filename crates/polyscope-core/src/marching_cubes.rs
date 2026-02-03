//! Marching Cubes isosurface extraction algorithm.
//!
//! Ported from the C++ `MarchingCubeCpp` library (public domain) used by C++ Polyscope.
//! Extracts a triangle mesh representing the isosurface of a 3D scalar field.

#![allow(
    clippy::unreadable_literal,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use glam::Vec3;

/// Output mesh from the marching cubes algorithm.
#[derive(Debug, Clone, Default)]
pub struct McmMesh {
    /// Interpolated vertex positions in grid-index space.
    pub vertices: Vec<Vec3>,
    /// Per-vertex normals (accumulated from adjacent face normals, then normalized).
    pub normals: Vec<Vec3>,
    /// Triangle indices (every 3 consecutive indices form a triangle).
    pub indices: Vec<u32>,
}

impl McmMesh {
    /// Returns the number of triangles in the mesh.
    #[must_use]
    pub fn num_triangles(&self) -> usize {
        self.indices.len() / 3
    }

    /// Returns true if the mesh has no triangles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Extracts the isosurface from a 3D scalar field using marching cubes.
///
/// # Arguments
/// * `field` - Scalar field values in C-contiguous order: the value for grid point
///   (ix, iy, iz) is stored at index `(ix * ny + iy) * nz + iz`.
/// * `isoval` - The isovalue defining the surface (surface is where `field == isoval`).
/// * `nx`, `ny`, `nz` - Grid dimensions (number of nodes in each direction).
///
/// # Returns
/// A mesh with vertices in grid-index space (integer coordinates interpolated along edges).
/// The caller must transform vertices to world space using grid spacing and origin.
///
/// # Panics
/// Panics if `field.len() != nx * ny * nz` or if any dimension is less than 2.
#[must_use]
pub fn marching_cubes(field: &[f32], isoval: f32, nx: u32, ny: u32, nz: u32) -> McmMesh {
    assert!(
        field.len() == (nx as usize) * (ny as usize) * (nz as usize),
        "Field size {} does not match dimensions {}x{}x{} = {}",
        field.len(),
        nx,
        ny,
        nz,
        (nx as usize) * (ny as usize) * (nz as usize)
    );
    assert!(nx >= 2 && ny >= 2 && nz >= 2, "All dimensions must be >= 2");

    let mut mesh = McmMesh {
        vertices: Vec::with_capacity(100_000),
        normals: Vec::with_capacity(100_000),
        indices: Vec::with_capacity(400_000),
    };

    let size = [nx, ny, nz];
    // Slab indices: stores vertex index for each of 3 edge axes at each (x, y) position
    // Uses modular z indexing (z % 2) to reuse memory between slabs
    let slab_len = (nx as usize) * (ny as usize) * 2;
    let mut slab_inds: Vec<[u32; 3]> = vec![[0; 3]; slab_len];

    let mut vs = [0.0_f32; 8];
    let mut edge_indices = [0_u32; 12];

    for z in 0..nz - 1 {
        for y in 0..ny - 1 {
            for x in 0..nx - 1 {
                // Evaluate 8 corner values, shifted by isoval
                vs[0] = -isoval + field[to_index_1d(x, y, z, &size)];
                vs[1] = -isoval + field[to_index_1d(x + 1, y, z, &size)];
                vs[2] = -isoval + field[to_index_1d(x, y + 1, z, &size)];
                vs[3] = -isoval + field[to_index_1d(x + 1, y + 1, z, &size)];
                vs[4] = -isoval + field[to_index_1d(x, y, z + 1, &size)];
                vs[5] = -isoval + field[to_index_1d(x + 1, y, z + 1, &size)];
                vs[6] = -isoval + field[to_index_1d(x, y + 1, z + 1, &size)];
                vs[7] = -isoval + field[to_index_1d(x + 1, y + 1, z + 1, &size)];

                // Build 8-bit configuration index from corner signs
                #[allow(clippy::cast_possible_truncation)]
                let config_n = (i32::from(vs[0] < 0.0))
                    | (i32::from(vs[1] < 0.0) << 1)
                    | (i32::from(vs[2] < 0.0) << 2)
                    | (i32::from(vs[3] < 0.0) << 3)
                    | (i32::from(vs[4] < 0.0) << 4)
                    | (i32::from(vs[5] < 0.0) << 5)
                    | (i32::from(vs[6] < 0.0) << 6)
                    | (i32::from(vs[7] < 0.0) << 7);

                // Skip fully inside or fully outside
                if config_n == 0 || config_n == 255 {
                    continue;
                }

                // Compute edge vertices (only for boundary edges not yet computed)
                // X-axis edges (axis=0)
                if y == 0 && z == 0 {
                    compute_edge(&mut slab_inds, &mut mesh, vs[0], vs[1], 0, x, y, z, &size);
                }
                if z == 0 {
                    compute_edge(
                        &mut slab_inds,
                        &mut mesh,
                        vs[2],
                        vs[3],
                        0,
                        x,
                        y + 1,
                        z,
                        &size,
                    );
                }
                if y == 0 {
                    compute_edge(
                        &mut slab_inds,
                        &mut mesh,
                        vs[4],
                        vs[5],
                        0,
                        x,
                        y,
                        z + 1,
                        &size,
                    );
                }
                compute_edge(
                    &mut slab_inds,
                    &mut mesh,
                    vs[6],
                    vs[7],
                    0,
                    x,
                    y + 1,
                    z + 1,
                    &size,
                );

                // Y-axis edges (axis=1)
                if x == 0 && z == 0 {
                    compute_edge(&mut slab_inds, &mut mesh, vs[0], vs[2], 1, x, y, z, &size);
                }
                if z == 0 {
                    compute_edge(
                        &mut slab_inds,
                        &mut mesh,
                        vs[1],
                        vs[3],
                        1,
                        x + 1,
                        y,
                        z,
                        &size,
                    );
                }
                if x == 0 {
                    compute_edge(
                        &mut slab_inds,
                        &mut mesh,
                        vs[4],
                        vs[6],
                        1,
                        x,
                        y,
                        z + 1,
                        &size,
                    );
                }
                compute_edge(
                    &mut slab_inds,
                    &mut mesh,
                    vs[5],
                    vs[7],
                    1,
                    x + 1,
                    y,
                    z + 1,
                    &size,
                );

                // Z-axis edges (axis=2)
                if x == 0 && y == 0 {
                    compute_edge(&mut slab_inds, &mut mesh, vs[0], vs[4], 2, x, y, z, &size);
                }
                if y == 0 {
                    compute_edge(
                        &mut slab_inds,
                        &mut mesh,
                        vs[1],
                        vs[5],
                        2,
                        x + 1,
                        y,
                        z,
                        &size,
                    );
                }
                if x == 0 {
                    compute_edge(
                        &mut slab_inds,
                        &mut mesh,
                        vs[2],
                        vs[6],
                        2,
                        x,
                        y + 1,
                        z,
                        &size,
                    );
                }
                compute_edge(
                    &mut slab_inds,
                    &mut mesh,
                    vs[3],
                    vs[7],
                    2,
                    x + 1,
                    y + 1,
                    z,
                    &size,
                );

                // Gather edge indices for this cell
                edge_indices[0] = slab_inds[to_index_1d_slab(x, y, z, &size)][0];
                edge_indices[1] = slab_inds[to_index_1d_slab(x, y + 1, z, &size)][0];
                edge_indices[2] = slab_inds[to_index_1d_slab(x, y, z + 1, &size)][0];
                edge_indices[3] = slab_inds[to_index_1d_slab(x, y + 1, z + 1, &size)][0];
                edge_indices[4] = slab_inds[to_index_1d_slab(x, y, z, &size)][1];
                edge_indices[5] = slab_inds[to_index_1d_slab(x + 1, y, z, &size)][1];
                edge_indices[6] = slab_inds[to_index_1d_slab(x, y, z + 1, &size)][1];
                edge_indices[7] = slab_inds[to_index_1d_slab(x + 1, y, z + 1, &size)][1];
                edge_indices[8] = slab_inds[to_index_1d_slab(x, y, z, &size)][2];
                edge_indices[9] = slab_inds[to_index_1d_slab(x + 1, y, z, &size)][2];
                edge_indices[10] = slab_inds[to_index_1d_slab(x, y + 1, z, &size)][2];
                edge_indices[11] = slab_inds[to_index_1d_slab(x + 1, y + 1, z, &size)][2];

                // Look up triangle configuration
                let config = MC_TRIS[config_n as usize];
                let n_triangles = (config & 0xF) as usize;
                let n_indices = n_triangles * 3;
                let index_base = mesh.indices.len();

                // Emit triangle indices
                let mut offset = 4;
                for _ in 0..n_indices {
                    let edge = ((config >> offset) & 0xF) as usize;
                    mesh.indices.push(edge_indices[edge]);
                    offset += 4;
                }

                // Accumulate face normals
                for i in 0..n_triangles {
                    let ia = mesh.indices[index_base + i * 3];
                    let ib = mesh.indices[index_base + i * 3 + 1];
                    let ic = mesh.indices[index_base + i * 3 + 2];
                    accumulate_normal(&mut mesh, ia, ib, ic);
                }
            }
        }
    }

    // Normalize all accumulated normals
    for normal in &mut mesh.normals {
        let len = normal.length();
        if len > 1e-10 {
            *normal /= len;
        }
    }

    mesh
}

/// Converts 3D grid coordinates to a 1D array index.
/// Layout: `(i * ny + j) * nz + k`
#[inline]
fn to_index_1d(i: u32, j: u32, k: u32, size: &[u32; 3]) -> usize {
    ((i as usize) * (size[1] as usize) + (j as usize)) * (size[2] as usize) + (k as usize)
}

/// Converts 3D coordinates to a slab index (modular z for memory reuse).
/// Layout: `nx * ny * (k % 2) + j * nx + i`
#[inline]
fn to_index_1d_slab(i: u32, j: u32, k: u32, size: &[u32; 3]) -> usize {
    (size[0] as usize) * (size[1] as usize) * ((k as usize) % 2)
        + (j as usize) * (size[0] as usize)
        + (i as usize)
}

/// Computes an edge vertex where the isosurface crosses, if the two endpoint values
/// have opposite signs. Stores the vertex index in the slab array.
#[inline]
fn compute_edge(
    slab_inds: &mut [[u32; 3]],
    mesh: &mut McmMesh,
    va: f32,
    vb: f32,
    axis: usize,
    x: u32,
    y: u32,
    z: u32,
    size: &[u32; 3],
) {
    // Only create vertex if sign differs (surface crosses this edge)
    if (va < 0.0) == (vb < 0.0) {
        return;
    }
    let mut v = Vec3::new(x as f32, y as f32, z as f32);
    v[axis] += va / (va - vb);
    let idx = mesh.vertices.len() as u32;
    slab_inds[to_index_1d_slab(x, y, z, size)][axis] = idx;
    mesh.vertices.push(v);
    mesh.normals.push(Vec3::ZERO);
}

/// Accumulates the geometric normal of triangle (a, b, c) to all three vertices.
#[inline]
fn accumulate_normal(mesh: &mut McmMesh, a: u32, b: u32, c: u32) {
    let va = mesh.vertices[a as usize];
    let vb = mesh.vertices[b as usize];
    let vc = mesh.vertices[c as usize];
    let ab = va - vb;
    let cb = vc - vb;
    let n = cb.cross(ab);
    mesh.normals[a as usize] += n;
    mesh.normals[b as usize] += n;
    mesh.normals[c as usize] += n;
}

/// Look-up table for triangle configurations (256 entries, one per cube configuration).
///
/// Each entry is a `u64` encoding:
/// - Bits `[3:0]`: Number of triangles (0-5)
/// - Bits `[7:4]`, `[11:8]`, ...: Edge indices (0-11) for each triangle vertex, 4 bits each
///
/// Ported from `MarchingCubeCpp` (public domain).
#[rustfmt::skip]
static MC_TRIS: [u64; 256] = [
    0, 33793, 36945, 159668546,
    18961, 144771090, 5851666, 595283255635,
    20913, 67640146, 193993474, 655980856339,
    88782242, 736732689667, 797430812739, 194554754,
    26657, 104867330, 136709522, 298069416227,
    109224258, 8877909667, 318136408323, 1567994331701604,
    189884450, 350847647843, 559958167731, 3256298596865604,
    447393122899, 651646838401572, 2538311371089956, 737032694307,
    29329, 43484162, 91358498, 374810899075,
    158485010, 178117478419, 88675058979, 433581536604804,
    158486962, 649105605635, 4866906995, 3220959471609924,
    649165714851, 3184943915608436, 570691368417972, 595804498035,
    124295042, 431498018963, 508238522371, 91518530,
    318240155763, 291789778348404, 1830001131721892, 375363605923,
    777781811075, 1136111028516116, 3097834205243396, 508001629971,
    2663607373704004, 680242583802939237, 333380770766129845, 179746658,
    42545, 138437538, 93365810, 713842853011,
    73602098, 69575510115, 23964357683, 868078761575828,
    28681778, 713778574611, 250912709379, 2323825233181284,
    302080811955, 3184439127991172, 1694042660682596, 796909779811,
    176306722, 150327278147, 619854856867, 1005252473234484,
    211025400963, 36712706, 360743481544788, 150627258963,
    117482600995, 1024968212107700, 2535169275963444, 4734473194086550421,
    628107696687956, 9399128243, 5198438490361643573, 194220594,
    104474994, 566996932387, 427920028243, 2014821863433780,
    492093858627, 147361150235284, 2005882975110676, 9671606099636618005,
    777701008947, 3185463219618820, 482784926917540, 2900953068249785909,
    1754182023747364, 4274848857537943333, 13198752741767688709, 2015093490989156,
    591272318771, 2659758091419812, 1531044293118596, 298306479155,
    408509245114388, 210504348563, 9248164405801223541, 91321106,
    2660352816454484, 680170263324308757, 8333659837799955077, 482966828984116,
    4274926723105633605, 3184439197724820, 192104450, 15217,
    45937, 129205250, 129208402, 529245952323,
    169097138, 770695537027, 382310500883, 2838550742137652,
    122763026, 277045793139, 81608128403, 1991870397907988,
    362778151475, 2059003085103236, 2132572377842852, 655681091891,
    58419234, 239280858627, 529092143139, 1568257451898804,
    447235128115, 679678845236084, 2167161349491220, 1554184567314086709,
    165479003923, 1428768988226596, 977710670185060, 10550024711307499077,
    1305410032576132, 11779770265620358997, 333446212255967269, 978168444447012,
    162736434, 35596216627, 138295313843, 891861543990356,
    692616541075, 3151866750863876, 100103641866564, 6572336607016932133,
    215036012883, 726936420696196, 52433666, 82160664963,
    2588613720361524, 5802089162353039525, 214799000387, 144876322,
    668013605731, 110616894681956, 1601657732871812, 430945547955,
    3156382366321172, 7644494644932993285, 3928124806469601813, 3155990846772900,
    339991010498708, 10743689387941597493, 5103845475, 105070898,
    3928064910068824213, 156265010, 1305138421793636, 27185,
    195459938, 567044449971, 382447549283, 2175279159592324,
    443529919251, 195059004769796, 2165424908404116, 1554158691063110021,
    504228368803, 1436350466655236, 27584723588724, 1900945754488837749,
    122971970, 443829749251, 302601798803, 108558722,
    724700725875, 43570095105972, 2295263717447940, 2860446751369014181,
    2165106202149444, 69275726195, 2860543885641537797, 2165106320445780,
    2280890014640004, 11820349930268368933, 8721082628082003989, 127050770,
    503707084675, 122834978, 2538193642857604, 10129,
    801441490467, 2923200302876740, 1443359556281892, 2901063790822564949,
    2728339631923524, 7103874718248233397, 12775311047932294245, 95520290,
    2623783208098404, 1900908618382410757, 137742672547, 2323440239468964,
    362478212387, 727199575803140, 73425410, 34337,
    163101314, 668566030659, 801204361987, 73030562,
    591509145619, 162574594, 100608342969108, 5553,
    724147968595, 1436604830452292, 176259090, 42001,
    143955266, 2385, 18433, 0,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_field() {
        // All values above isoval → no surface
        let field = vec![1.0; 3 * 3 * 3];
        let mesh = marching_cubes(&field, 0.0, 3, 3, 3);
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_constant_below() {
        // All values below isoval → no surface
        let field = vec![-1.0; 3 * 3 * 3];
        let mesh = marching_cubes(&field, 0.0, 3, 3, 3);
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_sphere_sdf() {
        // Sphere SDF: distance from center minus radius
        let n = 20_u32;
        let center = Vec3::splat(n as f32 / 2.0);
        let radius = n as f32 / 4.0;
        let mut field = vec![0.0_f32; (n * n * n) as usize];
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let p = Vec3::new(i as f32, j as f32, k as f32);
                    let dist = (p - center).length() - radius;
                    field[to_index_1d(i, j, k, &[n, n, n])] = dist;
                }
            }
        }

        let mesh = marching_cubes(&field, 0.0, n, n, n);

        // Should produce a reasonable number of triangles for a sphere
        assert!(
            mesh.num_triangles() > 100,
            "Expected >100 triangles, got {}",
            mesh.num_triangles()
        );
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.indices.len() % 3, 0);

        // All indices should be valid
        for &idx in &mesh.indices {
            assert!(
                (idx as usize) < mesh.vertices.len(),
                "Index {} out of range ({})",
                idx,
                mesh.vertices.len()
            );
        }

        // Normals should be unit length
        for normal in &mesh.normals {
            let len = normal.length();
            assert!((len - 1.0).abs() < 0.01, "Normal length {len} is not unit",);
        }

        // Vertices should be near the sphere surface (within grid spacing)
        for v in &mesh.vertices {
            let dist = (Vec3::new(v.z, v.y, v.x) - center).length(); // swizzle back
            assert!(
                (dist - radius).abs() < 2.0,
                "Vertex {v:?} is {dist} from sphere (radius {radius})",
            );
        }
    }

    #[test]
    fn test_single_crossing() {
        // 2x2x2 grid with one corner inside, rest outside
        let mut field = vec![1.0_f32; 8];
        field[0] = -1.0; // corner (0,0,0) is inside
        let mesh = marching_cubes(&field, 0.0, 2, 2, 2);

        // Should produce exactly 1 triangle (config = 1)
        assert_eq!(mesh.num_triangles(), 1);
        assert_eq!(mesh.indices.len(), 3);
    }

    #[test]
    #[should_panic(expected = "Field size")]
    fn test_wrong_field_size() {
        let field = vec![0.0; 10]; // wrong size for 3x3x3
        let _ = marching_cubes(&field, 0.0, 3, 3, 3);
    }

    #[test]
    #[should_panic(expected = "dimensions must be >= 2")]
    fn test_dimension_too_small() {
        let field = vec![0.0; 1];
        let _ = marching_cubes(&field, 0.0, 1, 1, 1);
    }
}
