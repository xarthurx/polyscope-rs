use crate::{Vec3, with_context_mut};

/// Registers a floating scalar image (not attached to any structure).
pub fn register_floating_scalar_image(
    name: impl Into<String>,
    width: u32,
    height: u32,
    values: Vec<f32>,
) {
    use polyscope_structures::floating::FloatingScalarImage;
    let img = FloatingScalarImage::new(name, width, height, values);
    with_context_mut(|ctx| {
        ctx.floating_quantities.push(Box::new(img));
    });
}

/// Registers a floating color image (not attached to any structure).
pub fn register_floating_color_image(
    name: impl Into<String>,
    width: u32,
    height: u32,
    colors: Vec<Vec3>,
) {
    use polyscope_structures::floating::FloatingColorImage;
    let img = FloatingColorImage::new(name, width, height, colors);
    with_context_mut(|ctx| {
        ctx.floating_quantities.push(Box::new(img));
    });
}

/// Removes a floating quantity by name.
pub fn remove_floating_quantity(name: &str) {
    with_context_mut(|ctx| {
        ctx.floating_quantities.retain(|q| q.name() != name);
    });
}

/// Removes all floating quantities.
pub fn remove_all_floating_quantities() {
    with_context_mut(|ctx| {
        ctx.floating_quantities.clear();
    });
}
