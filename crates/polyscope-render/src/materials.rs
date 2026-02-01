//! Material system for surface rendering.
//!
//! Materials define how surfaces are shaded. Blendable materials (clay, wax, candy, flat)
//! use 4-channel matcap textures (R/G/B/K) for color-tinted lighting. Static materials
//! (mud, ceramic, jade, normal) use a single matcap texture for all channels.

use std::collections::HashMap;

/// A material definition for rendering.
///
/// Materials control the appearance of surfaces. Blendable materials have separate
/// R/G/B/K matcap textures that are weighted by the surface color. Static materials
/// use a single matcap texture.
#[derive(Debug, Clone)]
pub struct Material {
    /// Material name.
    pub name: String,
    /// Whether this is a flat (unlit) material.
    pub is_flat: bool,
    /// Whether this material has separate R/G/B/K matcap channels (blendable).
    pub is_blendable: bool,
    /// Ambient light factor (0.0 - 1.0). Used as fallback if matcap not loaded.
    pub ambient: f32,
    /// Diffuse reflection factor (0.0 - 1.0). Used as fallback if matcap not loaded.
    pub diffuse: f32,
    /// Specular reflection intensity (0.0 - 1.0). Used as fallback if matcap not loaded.
    pub specular: f32,
    /// Specular shininess/exponent (higher = sharper highlights). Used as fallback.
    pub shininess: f32,
}

impl Material {
    /// Creates a new material with default properties (not blendable).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_flat: false,
            is_blendable: false,
            ambient: 0.2,
            diffuse: 0.7,
            specular: 0.3,
            shininess: 32.0,
        }
    }

    /// Creates a new blendable material with custom properties.
    pub fn blendable(
        name: impl Into<String>,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
    ) -> Self {
        Self {
            name: name.into(),
            is_flat: false,
            is_blendable: true,
            ambient,
            diffuse,
            specular,
            shininess,
        }
    }

    /// Creates a new static (non-blendable) material with custom properties.
    pub fn static_mat(
        name: impl Into<String>,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
    ) -> Self {
        Self {
            name: name.into(),
            is_flat: false,
            is_blendable: false,
            ambient,
            diffuse,
            specular,
            shininess,
        }
    }

    /// Creates a flat (unlit) material. Flat is blendable but shader skips matcap.
    pub fn flat(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_flat: true,
            is_blendable: true,
            ambient: 1.0,
            diffuse: 0.0,
            specular: 0.0,
            shininess: 1.0,
        }
    }

    /// Creates the "clay" material - matte, minimal specularity. Blendable.
    #[must_use]
    pub fn clay() -> Self {
        Self::blendable("clay", 0.25, 0.75, 0.1, 8.0)
    }

    /// Creates the "wax" material - slightly glossy, soft highlights. Blendable.
    #[must_use]
    pub fn wax() -> Self {
        Self::blendable("wax", 0.2, 0.7, 0.4, 16.0)
    }

    /// Creates the "candy" material - shiny, bright highlights. Blendable.
    #[must_use]
    pub fn candy() -> Self {
        Self::blendable("candy", 0.15, 0.6, 0.7, 64.0)
    }

    /// Creates the "ceramic" material - smooth, moderate gloss. Static.
    #[must_use]
    pub fn ceramic() -> Self {
        Self::static_mat("ceramic", 0.2, 0.65, 0.5, 32.0)
    }

    /// Creates the "jade" material - translucent appearance (simulated). Static.
    #[must_use]
    pub fn jade() -> Self {
        Self::static_mat("jade", 0.3, 0.6, 0.3, 24.0)
    }

    /// Creates the "mud" material - very matte, no specularity. Static.
    #[must_use]
    pub fn mud() -> Self {
        Self::static_mat("mud", 0.3, 0.7, 0.0, 1.0)
    }

    /// Creates the "normal" material - balanced properties. Static.
    #[must_use]
    pub fn normal() -> Self {
        Self::static_mat("normal", 0.2, 0.7, 0.3, 32.0)
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::clay()
    }
}

/// GPU-compatible material uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniforms {
    /// Ambient factor.
    pub ambient: f32,
    /// Diffuse factor.
    pub diffuse: f32,
    /// Specular intensity.
    pub specular: f32,
    /// Shininess exponent.
    pub shininess: f32,
}

impl From<&Material> for MaterialUniforms {
    fn from(mat: &Material) -> Self {
        Self {
            ambient: mat.ambient,
            diffuse: mat.diffuse,
            specular: mat.specular,
            shininess: mat.shininess,
        }
    }
}

impl Default for MaterialUniforms {
    fn default() -> Self {
        Self {
            ambient: 0.2,
            diffuse: 0.7,
            specular: 0.3,
            shininess: 32.0,
        }
    }
}

/// Pre-built GPU resources for a matcap material.
///
/// Each material has 4 texture views (R, G, B, K channels) and a shared sampler.
/// For blendable materials, each channel is a different texture.
/// For static materials, all 4 views point to the same single texture.
pub struct MatcapTextureSet {
    /// Texture view for the R channel.
    pub tex_r: wgpu::TextureView,
    /// Texture view for the G channel.
    pub tex_g: wgpu::TextureView,
    /// Texture view for the B channel.
    pub tex_b: wgpu::TextureView,
    /// Texture view for the K (remainder) channel.
    pub tex_k: wgpu::TextureView,
    /// Linear filtering sampler.
    pub sampler: wgpu::Sampler,
    /// Pre-built bind group for this material.
    pub bind_group: wgpu::BindGroup,
}

/// Registry for managing materials.
#[derive(Default)]
pub struct MaterialRegistry {
    materials: HashMap<String, Material>,
    default_material: String,
}

impl MaterialRegistry {
    /// Creates a new material registry with default materials.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            materials: HashMap::new(),
            default_material: "clay".to_string(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        // Register default materials (matching C++ Polyscope style)
        self.register(Material::clay());
        self.register(Material::wax());
        self.register(Material::candy());
        self.register(Material::ceramic());
        self.register(Material::jade());
        self.register(Material::mud());
        self.register(Material::normal());
        self.register(Material::flat("flat"));
    }

    /// Registers a material.
    pub fn register(&mut self, material: Material) {
        self.materials.insert(material.name.clone(), material);
    }

    /// Gets a material by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Material> {
        self.materials.get(name)
    }

    /// Returns true if a material with the given name is registered.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.materials.contains_key(name)
    }

    /// Gets the default material.
    #[must_use]
    pub fn default_material(&self) -> &Material {
        self.materials
            .get(&self.default_material)
            .unwrap_or_else(|| {
                self.materials
                    .values()
                    .next()
                    .expect("no materials registered")
            })
    }

    /// Sets the default material name.
    pub fn set_default(&mut self, name: &str) {
        if self.materials.contains_key(name) {
            self.default_material = name.to_string();
        }
    }

    /// Returns all material names, with built-in materials first in a stable order,
    /// followed by custom materials sorted alphabetically.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        const BUILTIN_ORDER: &[&str] = &[
            "clay", "wax", "candy", "flat", "mud", "ceramic", "jade", "normal",
        ];
        let mut names: Vec<&str> = Vec::new();
        // Built-ins first, in canonical order
        for &builtin in BUILTIN_ORDER {
            if self.materials.contains_key(builtin) {
                names.push(builtin);
            }
        }
        // Custom materials after built-ins, sorted alphabetically
        let mut custom: Vec<&str> = self
            .materials
            .keys()
            .map(String::as_str)
            .filter(|n| !BUILTIN_ORDER.contains(n))
            .collect();
        custom.sort();
        names.extend(custom);
        names
    }

    /// Returns the number of registered materials.
    #[must_use]
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Returns true if no materials are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

// Embedded matcap texture data (extracted from C++ Polyscope bindata).
// Blendable materials have 4 separate HDR files (R/G/B/K channels).
// Static materials have 1 JPEG file (reused for all 4 channels).
mod matcap_data {
    // Blendable: Clay
    pub const CLAY_R: &[u8] = include_bytes!("../data/matcaps/clay_r.hdr");
    pub const CLAY_G: &[u8] = include_bytes!("../data/matcaps/clay_g.hdr");
    pub const CLAY_B: &[u8] = include_bytes!("../data/matcaps/clay_b.hdr");
    pub const CLAY_K: &[u8] = include_bytes!("../data/matcaps/clay_k.hdr");

    // Blendable: Wax
    pub const WAX_R: &[u8] = include_bytes!("../data/matcaps/wax_r.hdr");
    pub const WAX_G: &[u8] = include_bytes!("../data/matcaps/wax_g.hdr");
    pub const WAX_B: &[u8] = include_bytes!("../data/matcaps/wax_b.hdr");
    pub const WAX_K: &[u8] = include_bytes!("../data/matcaps/wax_k.hdr");

    // Blendable: Candy
    pub const CANDY_R: &[u8] = include_bytes!("../data/matcaps/candy_r.hdr");
    pub const CANDY_G: &[u8] = include_bytes!("../data/matcaps/candy_g.hdr");
    pub const CANDY_B: &[u8] = include_bytes!("../data/matcaps/candy_b.hdr");
    pub const CANDY_K: &[u8] = include_bytes!("../data/matcaps/candy_k.hdr");

    // Blendable: Flat
    pub const FLAT_R: &[u8] = include_bytes!("../data/matcaps/flat_r.hdr");
    pub const FLAT_G: &[u8] = include_bytes!("../data/matcaps/flat_g.hdr");
    pub const FLAT_B: &[u8] = include_bytes!("../data/matcaps/flat_b.hdr");
    pub const FLAT_K: &[u8] = include_bytes!("../data/matcaps/flat_k.hdr");

    // Static: Mud, Ceramic, Jade, Normal (JPEG)
    pub const MUD: &[u8] = include_bytes!("../data/matcaps/mud.jpg");
    pub const CERAMIC: &[u8] = include_bytes!("../data/matcaps/ceramic.jpg");
    pub const JADE: &[u8] = include_bytes!("../data/matcaps/jade.jpg");
    pub const NORMAL: &[u8] = include_bytes!("../data/matcaps/normal.jpg");
}

/// Decode an embedded image (HDR or JPEG) into float RGBA pixel data.
///
/// Returns `(width, height, rgba_f32_pixels)` where pixels are laid out as
/// `[r, g, b, a, r, g, b, a, ...]` in linear float space.
fn decode_matcap_image(data: &[u8]) -> (u32, u32, Vec<f32>) {
    use image::GenericImageView;

    let img = image::load_from_memory(data).expect("Failed to decode matcap image");
    let (width, height) = img.dimensions();

    // Convert to Rgba32F
    let rgb32f = img.to_rgb32f();
    let pixels = rgb32f.as_raw();

    // Pad RGB -> RGBA with alpha=1.0
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for chunk in pixels.chunks(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(1.0);
    }

    (width, height, rgba)
}

/// Decode an image file from disk into float RGBA pixel data.
///
/// Returns `(width, height, rgba_f32_pixels)` where pixels are laid out as
/// `[r, g, b, a, r, g, b, a, ...]` in linear float space.
///
/// Supports any format the `image` crate can open: HDR, JPEG, PNG, EXR, etc.
pub fn decode_matcap_image_from_file(
    path: &std::path::Path,
) -> std::result::Result<(u32, u32, Vec<f32>), String> {
    use image::GenericImageView;

    let img = image::open(path)
        .map_err(|e| format!("failed to open '{}': {}", path.display(), e))?;
    let (width, height) = img.dimensions();

    if width == 0 || height == 0 {
        return Err(format!("image '{}' has zero dimensions", path.display()));
    }

    let rgb32f = img.to_rgb32f();
    let pixels = rgb32f.as_raw();

    // Pad RGB -> RGBA with alpha=1.0
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for chunk in pixels.chunks(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(1.0);
    }

    Ok((width, height, rgba))
}

/// Upload a decoded matcap image as a GPU texture.
pub fn upload_matcap_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    rgba_data: &[f32],
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Convert f32 -> f16 for upload
    let half_data: Vec<u16> = rgba_data
        .iter()
        .map(|&v| half::f16::from_f32(v).to_bits())
        .collect();

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&half_data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4 * 2), // 4 channels * 2 bytes per f16
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    texture
}

/// Create a linear filtering sampler for matcap textures.
pub fn create_matcap_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Matcap Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

/// Initialize all matcap textures and bind groups.
///
/// Returns a `HashMap` mapping material name -> `MatcapTextureSet`.
#[must_use] 
pub fn init_matcap_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> HashMap<String, MatcapTextureSet> {
    // Blendable material entry: (name, R channel, G channel, B channel, K channel)
    type BlendableMatEntry<'a> = (&'a str, &'a [u8], &'a [u8], &'a [u8], &'a [u8]);

    let sampler = create_matcap_sampler(device);
    let mut textures = HashMap::new();

    // Helper: decode + upload a single texture
    let upload = |label: &str, data: &[u8]| -> wgpu::Texture {
        let (w, h, rgba) = decode_matcap_image(data);
        upload_matcap_texture(device, queue, label, w, h, &rgba)
    };

    // Blendable materials: 4 separate textures (R, G, B, K channels)
    let blendable_mats: &[BlendableMatEntry<'_>] = &[
        (
            "clay",
            matcap_data::CLAY_R,
            matcap_data::CLAY_G,
            matcap_data::CLAY_B,
            matcap_data::CLAY_K,
        ),
        (
            "wax",
            matcap_data::WAX_R,
            matcap_data::WAX_G,
            matcap_data::WAX_B,
            matcap_data::WAX_K,
        ),
        (
            "candy",
            matcap_data::CANDY_R,
            matcap_data::CANDY_G,
            matcap_data::CANDY_B,
            matcap_data::CANDY_K,
        ),
        (
            "flat",
            matcap_data::FLAT_R,
            matcap_data::FLAT_G,
            matcap_data::FLAT_B,
            matcap_data::FLAT_K,
        ),
    ];

    for &(name, r_data, g_data, b_data, k_data) in blendable_mats {
        let tex_r = upload(&format!("matcap_{name}_r"), r_data);
        let tex_g = upload(&format!("matcap_{name}_g"), g_data);
        let tex_b = upload(&format!("matcap_{name}_b"), b_data);
        let tex_k = upload(&format!("matcap_{name}_k"), k_data);

        let view_r = tex_r.create_view(&wgpu::TextureViewDescriptor::default());
        let view_g = tex_g.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = tex_b.create_view(&wgpu::TextureViewDescriptor::default());
        let view_k = tex_k.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("matcap_{name}_bind_group")),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view_r),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_g),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&view_k),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        textures.insert(
            name.to_string(),
            MatcapTextureSet {
                tex_r: view_r,
                tex_g: view_g,
                tex_b: view_b,
                tex_k: view_k,
                sampler: create_matcap_sampler(device), // each set gets its own
                bind_group,
            },
        );
    }

    // Static materials: 1 texture reused for all 4 channels
    let static_mats: &[(&str, &[u8])] = &[
        ("mud", matcap_data::MUD),
        ("ceramic", matcap_data::CERAMIC),
        ("jade", matcap_data::JADE),
        ("normal", matcap_data::NORMAL),
    ];

    for &(name, data) in static_mats {
        let tex = upload(&format!("matcap_{name}"), data);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

        // For static materials, create 4 views from the same texture
        let view_r = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let view_g = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = tex.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("matcap_{name}_bind_group")),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_r),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view_g),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        textures.insert(
            name.to_string(),
            MatcapTextureSet {
                tex_r: tex.create_view(&wgpu::TextureViewDescriptor::default()),
                tex_g: tex.create_view(&wgpu::TextureViewDescriptor::default()),
                tex_b: tex.create_view(&wgpu::TextureViewDescriptor::default()),
                tex_k: tex.create_view(&wgpu::TextureViewDescriptor::default()),
                sampler: create_matcap_sampler(device),
                bind_group,
            },
        );
    }

    textures
}

/// Create the matcap bind group layout (5 entries: 4 textures + 1 sampler).
#[must_use] 
pub fn create_matcap_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Matcap Bind Group Layout"),
        entries: &[
            // Binding 0: mat_r texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Binding 1: mat_g texture
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Binding 2: mat_b texture
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Binding 3: mat_k texture
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Binding 4: sampler
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_material() {
        let mat = Material::flat("test_flat");
        assert!(mat.is_flat);
        assert!(mat.is_blendable);
        assert_eq!(mat.diffuse, 0.0);
        assert_eq!(mat.specular, 0.0);
    }

    #[test]
    fn test_material_registry() {
        let registry = MaterialRegistry::new();
        assert!(registry.get("clay").is_some());
        assert!(registry.get("wax").is_some());
        assert!(registry.get("candy").is_some());
        assert!(registry.get("flat").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_material_uniforms() {
        let mat = Material::candy();
        let uniforms = MaterialUniforms::from(&mat);
        assert_eq!(uniforms.ambient, mat.ambient);
        assert_eq!(uniforms.specular, mat.specular);
    }

    #[test]
    fn test_blendable_materials() {
        assert!(Material::clay().is_blendable);
        assert!(Material::wax().is_blendable);
        assert!(Material::candy().is_blendable);
        assert!(Material::flat("flat").is_blendable);
        assert!(!Material::mud().is_blendable);
        assert!(!Material::ceramic().is_blendable);
        assert!(!Material::jade().is_blendable);
        assert!(!Material::normal().is_blendable);
    }

    #[test]
    fn test_material_registry_has() {
        let registry = MaterialRegistry::new();
        assert!(registry.has("clay"));
        assert!(registry.has("wax"));
        assert!(registry.has("normal"));
        assert!(!registry.has("nonexistent"));
        assert!(!registry.has("my_custom"));
    }

    #[test]
    fn test_material_registry_names_order() {
        let registry = MaterialRegistry::new();
        let names = registry.names();
        // Built-ins should appear in canonical order
        assert_eq!(
            names,
            vec!["clay", "wax", "candy", "flat", "mud", "ceramic", "jade", "normal"]
        );
    }

    #[test]
    fn test_material_registry_custom() {
        let mut registry = MaterialRegistry::new();
        let mut custom = Material::clay();
        custom.name = "zebra_mat".to_string();
        registry.register(custom);

        let mut custom2 = Material::clay();
        custom2.name = "alpha_mat".to_string();
        registry.register(custom2);

        assert!(registry.has("zebra_mat"));
        assert!(registry.has("alpha_mat"));

        let names = registry.names();
        // Built-ins first in canonical order, then custom sorted alphabetically
        let expected = vec![
            "clay", "wax", "candy", "flat", "mud", "ceramic", "jade", "normal",
            "alpha_mat", "zebra_mat",
        ];
        assert_eq!(names, expected);
    }
}
