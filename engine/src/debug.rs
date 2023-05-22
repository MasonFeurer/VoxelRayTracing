use crate::gpu::shaders::Settings as ShaderSettings;
use crate::State;
use egui::*;
use glam::Vec3;

pub fn debug_ui(state: &mut State, ui: &mut Ui) {
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
    fn toggle(ui: &mut Ui, label: &str, v: &mut u32) -> bool {
        ui.add_space(SPACING);
        let mut b = *v == 1;
        let result = ui.checkbox(&mut b, label).changed();
        *v = b as u32;
        result
    }
    fn label(ui: &mut Ui, label: &str, color: Color32) {
        ui.label(RichText::new(label).color(color));
    }

    let in_hand = state.voxel_in_hand;
    let red = Color32::from_rgb(255, 150, 0);
    let green = Color32::from_rgb(0, 255, 0);
    let blue = Color32::from_rgb(0, 255, 255);
    let white = Color32::WHITE;

    ui.add_space(3.0);
    label(ui, &format!("fps: {}", state.fps), white);
    ui.add_space(3.0);
    label(ui, &format!("in hand: {:?}", in_hand.display_name()), white);
    ui.add_space(3.0);
    label(ui, &format!("on ground: {}", state.player.on_ground), white);

    ui.add_space(3.0);
    label(ui, &format!("X: {:#}", state.player.pos.x), red);
    label(ui, &format!("Y: {:#}", state.player.pos.y), green);
    label(ui, &format!("Z: {:#}", state.player.pos.z), blue);

    value_f32(ui, "speed", &mut state.player.speed, 0.1, 3.0);

    ui.separator();

    ui.collapsing("world", |ui| {
        value_u32(ui, "world depth", &mut state.world_depth, 2, 11);
        value_f32(ui, "terrain scale", &mut state.world_gen.scale, 0.1, 10.0);
        value_f32(ui, "terrain freq", &mut state.world_gen.freq, 0.1, 10.0);

        if ui.button("regenerate").clicked() {
            state.world.set_max_depth(state.world_depth);
            state.world.clear();
            _ = state.world.populate_with(&state.world_gen);
            state
                .shaders
                .raytracer
                .world
                .write(&state.gpu.queue, &state.world);
        }
    });

    ui.separator();

    let mut changed = false;
    ui.collapsing("shader", |ui| {
        let ShaderSettings {
            samples_per_pixel,
            max_ray_steps,
            max_ray_bounces,
            sky_color,
            sun_pos,
            ..
        } = &mut state.settings;

        changed |= value_u32(ui, "max ray steps", max_ray_steps, 0, 300);
        changed |= value_u32(ui, "max ray bounces", max_ray_bounces, 0, 30);
        changed |= value_u32(ui, "samples/pixel", samples_per_pixel, 1, 30);
        changed |= color_picker(ui, "sky color", sky_color);
        if value_f32(ui, "sun pos", &mut state.sun_angle, 0.0, 360.0) {
            changed = true;
            *sun_pos = Vec3::new(
                state.sun_angle.to_radians().sin() * 500.0,
                state.sun_angle.to_radians().cos() * 500.0,
                state.world.size as f32 * 0.5,
            )
            .to_array();
            state.resize_output_tex = true;
        }
        if value_u32(ui, "vertical samples", &mut state.output_tex_h, 50, 2000) {
            state.resize_output_tex = true;
        }
    });

    if changed {
        state
            .shaders
            .raytracer
            .settings
            .write(&state.gpu.queue, &state.settings);
    }
}
