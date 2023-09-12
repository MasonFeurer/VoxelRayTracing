use crate::gpu::Settings as ShaderSettings;
use crate::world::Material;
use crate::{FrameInput, GameState, UpdateResult};
use egui::*;
use glam::Vec3;

#[derive(Default)]
pub struct UiResult {
    pub clear_result: bool,
}

pub fn draw_ui(
    state: &mut GameState,
    frame_i: &FrameInput,
    update: &UpdateResult,
    ctx: &Context,
) -> UiResult {
    let mut style: Style = (*ctx.style()).clone();
    style.visuals.widgets.noninteractive.fg_stroke.color = Color32::WHITE;
    style.visuals.widgets.noninteractive.bg_stroke.color = Color32::WHITE;
    style.visuals.widgets.inactive.fg_stroke.color = Color32::WHITE;
    style.visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    style.visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
    ctx.set_style(style);

    let mut frame = containers::Frame::side_top_panel(&ctx.style());
    frame.fill = frame.fill.linear_multiply(0.9);

    let mut result = UiResult::default();
    egui::SidePanel::left("left").frame(frame).show(ctx, |ui| {
        left_panel_ui(state, frame_i, update, ui, &mut result);
    });
    result
}

fn left_panel_ui(
    state: &mut GameState,
    frame: &FrameInput,
    _update: &UpdateResult,
    ui: &mut Ui,
    result: &mut UiResult,
) {
    const SPACING: f32 = 5.0;
    fn value_f32(ui: &mut Ui, label: &str, v: &mut f32, min: f32, max: f32) -> bool {
        ui.add_space(SPACING);
        ui.label(label);
        ui.add(Slider::new(v, min..=max)).changed()
    }
    fn value_u32(ui: &mut Ui, label: &str, v: &mut u32, min: u32, max: u32) -> bool {
        ui.add_space(SPACING);
        ui.label(label);
        ui.add(Slider::new(v, min..=max)).changed()
    }
    fn color_picker(ui: &mut Ui, label: &str, color: &mut [f32; 3]) -> bool {
        let mut rgba = [color[0], color[1], color[2], 1.0];
        ui.add_space(SPACING);
        ui.label(label);
        let r = ui.color_edit_button_rgba_premultiplied(&mut rgba).changed();
        color.clone_from_slice(&rgba[0..3]);
        r
    }
    #[allow(dead_code)]
    fn toggle_bool(ui: &mut Ui, label: &str, v: &mut bool) -> bool {
        ui.add_space(SPACING);
        let result = ui.checkbox(v, label).changed();
        result
    }
    #[allow(dead_code)]
    fn toggle_u32(ui: &mut Ui, label: &str, v: &mut u32) -> bool {
        ui.add_space(SPACING);
        let mut b = *v == 1;
        let result = ui.checkbox(&mut b, label).changed();
        *v = b as u32;
        result
    }
    fn label(ui: &mut Ui, label: &str, color: Color32) {
        ui.label(RichText::new(label).color(color));
    }

    let in_hand = crate::INVENTORY[state.inv_sel as usize];
    let red = Color32::from_rgb(255, 150, 0);
    let green = Color32::from_rgb(0, 255, 0);
    let blue = Color32::from_rgb(0, 255, 255);
    let white = Color32::WHITE;

    ui.add_space(3.0);
    label(ui, &format!("fps: {}", frame.fps), white);
    ui.add_space(3.0);
    label(ui, &format!("place: {:?}", in_hand.display_name()), white);
    ui.add_space(3.0);

    let pos = state.player.pos;

    ui.add_space(3.0);
    label(ui, &format!("X: {:#}", pos.x), red);
    label(ui, &format!("Y: {:#}", pos.y), green);
    label(ui, &format!("Z: {:#}", pos.z), blue);

    value_f32(ui, "speed", &mut state.player.speed, 0.1, 3.0);
    let used_gpu_mem =
        ((state.world.last_used_node() as f64 / state.gpu_res.buffers.nodes.count as f64) * 100.0)
            .round() as u32;
    label(ui, &format!("GPU memory filled: {used_gpu_mem}%"), red);

    ui.separator();
    let mut changed = false;

    ui.collapsing("shader", |ui| {
        let ShaderSettings {
            max_ray_bounces,
            sky_color,
            sun_pos,
            sun_intensity,
            ..
        } = &mut state.settings;

        toggle_bool(ui, "path tracing", &mut state.path_tracing);
        changed |= value_u32(ui, "max ray bounces", max_ray_bounces, 0, 30);
        changed |= color_picker(ui, "sky color", sky_color);
        changed |= value_f32(ui, "sun intensity", sun_intensity, 0.0, 100.0);
        if value_f32(ui, "sun pos", &mut state.sun_angle, 0.0, 360.0) {
            changed = true;
            *sun_pos = Vec3::new(
                state.sun_angle.to_radians().sin() * 500.0,
                state.sun_angle.to_radians().cos() * 500.0,
                state.world.size as f32 * 0.5,
            )
            .to_array();
            result.clear_result = true;
        }
        if value_u32(
            ui,
            "vertical samples",
            &mut state.vertical_samples,
            50,
            2000,
        ) {
            result.clear_result = true;
        }
    });

    ui.separator();

    ui.collapsing("visuals", |ui| {
        let mut changed2 = false;

        let Material {
            color,
            scatter,
            emission,
            polish_bounce_chance,
            polish_color,
            polish_scatter,
            ..
        } = &mut state.voxel_materials[in_hand.0 as usize];

        changed2 |= value_f32(ui, "scatter", scatter, 0.0, 1.0);
        changed2 |= value_f32(ui, "emission", emission, 0.0, 10.0);
        changed2 |= value_f32(ui, "polish bounce chance", polish_bounce_chance, 0.0, 1.0);
        changed2 |= value_f32(ui, "polish scatter", polish_scatter, 0.0, 1.0);
        changed2 |= color_picker(ui, "color", color);
        changed2 |= color_picker(ui, "polish color", polish_color);

        if changed2 {
            state.gpu_res.buffers.voxel_materials.write_slice(
                &state.gpu,
                0,
                &state.voxel_materials,
            );
        }
    });

    if changed {
        let settings = &state.settings;
        state.gpu_res.buffers.settings.write(&state.gpu, settings);
    }
}
