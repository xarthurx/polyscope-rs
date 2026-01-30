//! Structure-specific UI builders.

use egui::Ui;

/// Available material names for matcap rendering.
const MATERIALS: &[&str] = &["clay", "wax", "candy", "flat", "mud", "ceramic", "jade", "normal"];

/// Builds a material selector ComboBox. Returns true if the material changed.
pub fn build_material_selector(ui: &mut Ui, material: &mut String) -> bool {
    let mut changed = false;
    egui::ComboBox::from_label("Material")
        .selected_text(material.as_str())
        .show_ui(ui, |ui| {
            for &mat in MATERIALS {
                if ui.selectable_value(material, mat.to_string(), mat).changed() {
                    changed = true;
                }
            }
        });
    changed
}

/// Builds UI for a point cloud.
pub fn build_point_cloud_ui(
    ui: &mut Ui,
    num_points: usize,
    point_radius: &mut f32,
    base_color: &mut [f32; 3],
    material: &mut String,
) -> bool {
    let mut changed = false;

    ui.label(format!("Points: {num_points}"));

    if build_material_selector(ui, material) {
        changed = true;
    }

    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(base_color).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Radius:");
        if ui
            .add(
                egui::DragValue::new(point_radius)
                    .speed(0.001)
                    .range(0.001..=0.5),
            )
            .changed()
        {
            changed = true;
        }
    });

    changed
}

/// Builds UI for a surface mesh.
pub fn build_surface_mesh_ui(
    ui: &mut Ui,
    num_vertices: usize,
    num_faces: usize,
    num_edges: usize,
    shade_style: &mut u32,
    surface_color: &mut [f32; 3],
    transparency: &mut f32,
    show_edges: &mut bool,
    edge_width: &mut f32,
    edge_color: &mut [f32; 3],
    backface_policy: &mut u32,
    material: &mut String,
) -> bool {
    let mut changed = false;

    ui.label(format!("Vertices: {num_vertices}"));
    ui.label(format!("Faces: {num_faces}"));
    ui.label(format!("Edges: {num_edges}"));

    if build_material_selector(ui, material) {
        changed = true;
    }

    ui.separator();

    // Shade style
    egui::ComboBox::from_label("Shading")
        .selected_text(match *shade_style {
            0 => "Smooth",
            1 => "Flat",
            _ => "Tri-Flat",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(shade_style, 0, "Smooth").changed() {
                changed = true;
            }
            if ui.selectable_value(shade_style, 1, "Flat").changed() {
                changed = true;
            }
            if ui.selectable_value(shade_style, 2, "Tri-Flat").changed() {
                changed = true;
            }
        });

    // Surface color
    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(surface_color).changed() {
            changed = true;
        }
    });

    // Opacity (displayed as 1.0 - transparency so slider semantics match the label:
    // opacity 1 = fully opaque, opacity 0 = fully transparent)
    ui.horizontal(|ui| {
        ui.label("Opacity:");
        let mut opacity = 1.0 - *transparency;
        if ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0)).changed() {
            *transparency = 1.0 - opacity;
            changed = true;
        }
    });

    ui.separator();

    // Wireframe
    if ui.checkbox(show_edges, "Show edges").changed() {
        changed = true;
    }

    if *show_edges {
        ui.indent("edges", |ui| {
            ui.horizontal(|ui| {
                ui.label("Width:");
                if ui
                    .add(egui::DragValue::new(edge_width).speed(0.1).range(0.1..=5.0))
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Color:");
                if ui.color_edit_button_rgb(edge_color).changed() {
                    changed = true;
                }
            });
        });
    }

    ui.separator();

    // Backface policy
    egui::ComboBox::from_label("Backface")
        .selected_text(match *backface_policy {
            0 => "Identical",
            1 => "Different",
            2 => "Custom",
            _ => "Cull",
        })
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(backface_policy, 0, "Identical")
                .changed()
            {
                changed = true;
            }
            if ui
                .selectable_value(backface_policy, 1, "Different")
                .changed()
            {
                changed = true;
            }
            if ui.selectable_value(backface_policy, 2, "Custom").changed() {
                changed = true;
            }
            if ui.selectable_value(backface_policy, 3, "Cull").changed() {
                changed = true;
            }
        });

    changed
}

/// Builds UI for a curve network.
pub fn build_curve_network_ui(
    ui: &mut Ui,
    num_nodes: usize,
    num_edges: usize,
    radius: &mut f32,
    radius_is_relative: &mut bool,
    color: &mut [f32; 3],
    render_mode: &mut u32,
    material: &mut String,
) -> bool {
    let mut changed = false;

    ui.label(format!("Nodes: {num_nodes}"));
    ui.label(format!("Edges: {num_edges}"));

    if build_material_selector(ui, material) {
        changed = true;
    }

    ui.separator();

    // Color
    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(color).changed() {
            changed = true;
        }
    });

    // Render mode
    egui::ComboBox::from_label("Render")
        .selected_text(match *render_mode {
            0 => "Lines",
            _ => "Tubes",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(render_mode, 0, "Lines").changed() {
                changed = true;
            }
            if ui.selectable_value(render_mode, 1, "Tubes").changed() {
                changed = true;
            }
        });

    // Radius
    ui.horizontal(|ui| {
        ui.label("Radius:");
        if ui
            .add(
                egui::DragValue::new(radius)
                    .speed(0.001)
                    .range(0.001..=10.0),
            )
            .changed()
        {
            changed = true;
        }
    });

    // Radius is relative checkbox
    if ui.checkbox(radius_is_relative, "Relative radius").changed() {
        changed = true;
    }

    changed
}
