#![allow(dead_code)]
#![feature(new_uninit)]
#![feature(let_chains)]
// TODO: try to remove feature flags

pub mod gpu;
pub mod input;
pub mod math;
pub mod player;
pub mod world;

use crate::gpu::shaders::{Settings, Shaders};
use crate::gpu::{debug::Egui, ColoredMesh, Gpu, GpuMesh};
use crate::input::{InputState, Key};
use crate::math::dda::HitResult;
use crate::player::Player;
use crate::world::{Voxel, World};
use glam::{Mat4, UVec2, Vec3};
use std::time::SystemTime;
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window as WinitWindow;
use winit::window::{CursorGrabMode, Fullscreen, WindowBuilder};

struct Window {
    winit: WinitWindow,
    cursor_locked: bool,
}
impl Window {
    fn new(event_loop: &EventLoop<()>) -> Self {
        let winit = WindowBuilder::new()
            .with_title("Voxel Ray Tracing")
            .build(&event_loop)
            .unwrap();

        Self {
            winit,
            cursor_locked: false,
        }
    }

    fn size(&self) -> UVec2 {
        <[u32; 2]>::from(self.winit.inner_size()).into()
    }

    fn set_fullscreen(&mut self, fs: bool) {
        self.winit.set_fullscreen(match fs {
            false => None,
            true => Some(Fullscreen::Borderless(None)),
        });
    }
    fn toggle_fullscreen(&mut self) {
        self.winit.set_fullscreen(match self.winit.fullscreen() {
            Some(_) => None,
            None => Some(Fullscreen::Borderless(None)),
        });
    }
    fn set_cursor_locked(&mut self, locked: bool) {
        if locked == self.cursor_locked {
            return;
        }
        let grab_mode = match (locked, cfg!(target_os = "macos")) {
            (false, _) => CursorGrabMode::None,
            (_, true) => CursorGrabMode::Locked,
            (_, false) => CursorGrabMode::Confined,
        };
        if let Err(err) = self.winit.set_cursor_grab(grab_mode) {
            println!("error locking cursor: {err:?}");
        }
        self.winit.set_cursor_visible(!locked);
        self.cursor_locked = locked;
    }
    fn toggle_cursor_locked(&mut self) {
        self.set_cursor_locked(!self.cursor_locked)
    }
}

fn create_svo_mesh(gpu: &Gpu, _world: &World) -> GpuMesh {
    fn show_node(
        out: &mut ColoredMesh,
        world: &World,
        node_idx: u32,
        depth: u32,
        center: Vec3,
        size: f32,
    ) {
        let node = world.get_node(node_idx);
        let c = depth as f32 / 8.0;
        out.cube_frame(center - size * 0.5, center + size * 0.5, [c, c, c, 1.0]);
        if !node.is_split() {
            return;
        }
        for idx in 0..8 {
            let dir = Vec3::new(
                (((idx & 0b001) >> 0) * 2) as f32 - 1.0,
                (((idx & 0b010) >> 1) * 2) as f32 - 1.0,
                (((idx & 0b100) >> 2) * 2) as f32 - 1.0,
            );
            let center = center + dir * (size * 0.25);
            show_node(
                out,
                world,
                node.get_child(idx),
                depth + 1,
                center,
                size * 0.5,
            )
        }
    }

    let out = ColoredMesh::default();
    // show_node(
    //     &mut out,
    //     world,
    //     world.root_idx,
    //     0,
    //     Vec3::splat(world.size as f32 * 0.5),
    //     world.size as f32,
    // );
    out.upload(gpu)
}

struct State {
    window: Window,
    gpu: Gpu,
    shaders: Shaders,
    settings: Settings,

    player: Player,
    hit_result: Option<HitResult>,
    world: Box<World>,
    voxel_in_hand: Voxel,
    last_second: SystemTime,
    fps: u32,
    fps_temp: u32,
    svo_mesh: GpuMesh,
    show_svo: bool,
}
impl State {
    fn new(window: Window, gpu: Gpu) -> Self {
        let settings = Settings::default();

        let mut world = unsafe {
            let world = Box::<World>::new_zeroed();
            world.assume_init()
        };
        world.init(10);
        world.populate();

        let aspect = window.size().x as f32 / window.size().y as f32;

        let player = Player::new(Vec3::new(10.0, 40.0, 10.0), 3.2);

        let shaders = Shaders::new(&gpu.device, gpu.surface_config.format, window.size());
        shaders.raytracer.world.write(&gpu.queue, &world);
        shaders.raytracer.settings.write(&gpu.queue, &settings);
        shaders
            .color_shader
            .model_mat
            .write(&gpu.queue, &Mat4::IDENTITY);
        shaders
            .color_shader
            .proj_mat
            .write(&gpu.queue, &player.create_proj_mat(aspect));

        let svo_mesh = create_svo_mesh(&gpu, &world);

        Self {
            window,
            gpu,
            shaders,
            settings,

            player,
            hit_result: None,
            world,
            voxel_in_hand: Voxel::DIRT,
            last_second: SystemTime::now(),
            fps: 0,
            fps_temp: 0,
            svo_mesh,
            show_svo: false,
        }
    }

    fn update(&mut self, input: &InputState) {
        if self.window.cursor_locked {
            self.player.update(1.0, input, &self.world);

            let size = self.shaders.output_texture.size().as_vec2();
            self.shaders
                .raytracer
                .cam_data
                .write(&self.gpu.queue, &self.player.create_cam_data(size));
            self.shaders
                .color_shader
                .view_mat
                .write(&self.gpu.queue, &self.player.create_inv_view_mat());
        }
        if input.key_pressed(Key::T) {
            self.window.toggle_cursor_locked();
        }
        if input.key_pressed(Key::F) {
            self.window.toggle_fullscreen();
        }
        let mut rand_floats = [0.0; 128];
        for i in 0..rand_floats.len() {
            rand_floats[i] = fastrand::f32();
        }
        self.shaders
            .raytracer
            .rand_src
            .write(&self.gpu.queue, &rand_floats);

        self.hit_result = self.player.cast_ray(&self.world);

        if input.key_pressed(Key::Key1) {
            self.voxel_in_hand = Voxel::DIRT;
        }
        if input.key_pressed(Key::Key2) {
            self.voxel_in_hand = Voxel::GRASS;
        }
        if input.key_pressed(Key::Key3) {
            self.voxel_in_hand = Voxel::STONE;
        }
        if input.key_pressed(Key::Key4) {
            self.voxel_in_hand = Voxel::GOLD;
        }
        if input.key_pressed(Key::Key5) {
            self.voxel_in_hand = Voxel::MIRROR;
        }
        if input.key_pressed(Key::Key6) {
            self.voxel_in_hand = Voxel::WATER;
        }
        if input.key_pressed(Key::Key7) {
            self.voxel_in_hand = Voxel::FIRE;
        }

        if !self.window.cursor_locked {
            return;
        }

        if let Some(hit) = self.hit_result && input.left_button_pressed() {
            self.world.set_voxel(hit.pos, Voxel::AIR);
            self.shaders.raytracer.world.write(&self.gpu.queue, &self.world);
            self.svo_mesh = create_svo_mesh(&self.gpu, &self.world);
        }
        if let Some(hit) = self.hit_result && input.right_button_pressed() {
            self.world.set_voxel(hit.pos + hit.face, self.voxel_in_hand);
            self.shaders.raytracer.world.write(&self.gpu.queue, &self.world);
            self.svo_mesh = create_svo_mesh(&self.gpu, &self.world);
        }
    }

    fn resize(&mut self, new_size: UVec2) {
        self.gpu.resize(new_size);

        let proj_size = self.shaders.output_texture.size().as_vec2();
        let aspect = proj_size.x / proj_size.y;
        self.shaders
            .raytracer
            .cam_data
            .write(&self.gpu.queue, &self.player.create_cam_data(proj_size));
        self.shaders
            .color_shader
            .proj_mat
            .write(&self.gpu.queue, &self.player.create_proj_mat(aspect));
    }

    fn debug_ui(&mut self, ui: &mut egui::Ui) {
        use egui::*;

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
        fn color_picker(ui: &mut Ui, label: &str, color: &mut [f32; 4]) -> bool {
            ui.add_space(SPACING);
            ui.label(label);
            ui.color_edit_button_rgba_premultiplied(color).changed()
        }
        fn toggle_bool(ui: &mut Ui, label: &str, v: &mut bool) -> bool {
            ui.add_space(SPACING);
            let result = ui.checkbox(v, label).changed();
            result
        }
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

        let in_hand = self.voxel_in_hand;
        let [red, green, blue, white] = [
            Color32::from_rgb(255, 150, 0),
            Color32::from_rgb(0, 255, 0),
            Color32::from_rgb(0, 255, 255),
            Color32::WHITE,
        ];

        ui.add_space(3.0);
        label(ui, &format!("fps: {}", self.fps), white);
        ui.add_space(3.0);
        label(ui, &format!("in hand: {:?}", in_hand.display()), white);
        ui.add_space(3.0);
        label(ui, &format!("on ground: {}", self.player.on_ground), white);

        ui.add_space(3.0);
        label(ui, &format!("X: {:#}", self.player.pos.x), red);
        label(ui, &format!("Y: {:#}", self.player.pos.y), green);
        label(ui, &format!("Z: {:#}", self.player.pos.z), blue);

        ui.separator();
        ui.heading("shader");

        let Settings {
            max_ray_steps,
            sky_color,
            max_reflections,
            shadows,
            ..
        } = &mut self.settings;

        let mut changed = false;
        changed |= toggle_bool(ui, "sho svo", &mut self.show_svo);
        changed |= value_u32(ui, "ray max steps", max_ray_steps, 0, 300);
        changed |= value_u32(ui, "max reflections", max_reflections, 0, 100);
        changed |= toggle(ui, "shadows", shadows);
        changed |= color_picker(ui, "sky color", sky_color);

        value_f32(ui, "speed", &mut self.player.speed, 0.1, 3.0);

        if changed {
            self.shaders
                .raytracer
                .settings
                .write(&self.gpu.queue, &self.settings);
        }
    }

    fn render(&mut self, egui: &mut Egui) -> Result<(), wgpu::SurfaceError> {
        // --- get surface and create encoder ---
        let output = self.gpu.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("#encoder"),
            });

        // --- raytracer pass ---
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("#raytracer-pass"),
            });
            pass.set_pipeline(&self.shaders.raytracer.pipeline);
            pass.set_bind_group(0, &self.shaders.raytracer.bind_group, &[]);

            let [buffer_w, buffer_h]: [u32; 2] = self.shaders.output_texture.size().into();
            pass.dispatch_workgroups(buffer_w / 8, buffer_h / 8, 1);
        }

        // --- output tex shader pass ---
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("#output-tex-shader-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            pass.set_pipeline(&self.shaders.output_tex_shader.pipeline);
            pass.set_bind_group(0, &self.shaders.output_tex_shader.bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        // --- draw SVO mesh with color shader ---
        if self.show_svo {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("#svo-mesh-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            pass.set_pipeline(&self.shaders.color_shader.pipeline);
            pass.set_bind_group(0, &self.shaders.color_shader.bind_group, &[]);
            pass.set_vertex_buffer(0, self.svo_mesh.vertex_buf.slice(..));
            pass.set_index_buffer(self.svo_mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.svo_mesh.index_count, 0, 0..1);
        }

        // --- egui ---
        let egui_textures_free = {
            // --- create scene ---
            let egui_input = egui.winit.take_egui_input(&self.window.winit);
            let egui_output = egui.ctx.run(egui_input, |ctx| {
                let mut style: egui::Style = (*ctx.style()).clone();
                style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE;
                style.visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::WHITE;
                style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::WHITE;
                style.visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
                style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
                ctx.set_style(style);

                let mut frame = egui::containers::Frame::side_top_panel(&ctx.style());
                frame.fill = frame.fill.linear_multiply(0.9);

                egui::SidePanel::left("top").frame(frame).show(&ctx, |ui| {
                    self.debug_ui(ui);
                });
            });
            let egui_prims = egui.ctx.tessellate(egui_output.shapes);
            let screen_desc = egui_wgpu::renderer::ScreenDescriptor {
                size_in_pixels: self.shaders.output_texture.size().into(),
                pixels_per_point: egui_winit::native_pixels_per_point(&self.window.winit),
            };

            // --- update buffers ---
            for (id, image) in egui_output.textures_delta.set {
                egui.wgpu
                    .update_texture(&self.gpu.device, &self.gpu.queue, id, &image);
            }
            egui.wgpu.update_buffers(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &egui_prims,
                &screen_desc,
            );

            // --- render pass ---
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("#egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            egui.wgpu.render(&mut pass, &egui_prims, &screen_desc);

            egui_output.textures_delta.free
        };

        // --- submit passes ---
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // --- free egui textures ---
        for id in egui_textures_free {
            egui.wgpu.free_texture(&id);
        }

        // --- update fps counter ---
        self.fps_temp += 1;
        let now = SystemTime::now();
        if now.duration_since(self.last_second).unwrap().as_secs() >= 1 {
            self.last_second = now;
            self.fps = self.fps_temp;
            self.fps_temp = 0;
        }

        Ok(())
    }
}

pub fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();

    let mut input = input::InputState::default();

    let mut window = Window::new(&event_loop);
    window.set_cursor_locked(true);

    let gpu = pollster::block_on(Gpu::new(&window.winit));

    let mut egui = Egui::new(&event_loop, &gpu);
    egui.winit
        .set_pixels_per_point(egui_winit::native_pixels_per_point(&window.winit));
    let mut state = State::new(window, gpu);
    let mut last_frame = SystemTime::now();

    event_loop.run(move |event, _, flow| match event {
        event if input.update(&event) => {}
        Event::WindowEvent { event, .. } => match event {
            e if egui.winit.on_event(&egui.ctx, &e).consumed => {}
            WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
            WindowEvent::Resized(size) => {
                state.resize(UVec2::new(size.width, size.height));
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                let size = *new_inner_size;
                state.resize(UVec2::new(size.width, size.height));
            }
            _ => {}
        },
        Event::RedrawRequested(_) => {
            if SystemTime::now()
                .duration_since(last_frame)
                .unwrap()
                .as_millis()
                < (1000 / 60)
            {
                return;
            }
            last_frame = SystemTime::now();

            state.update(&input);

            input.finish_frame();
            match state.render(&mut egui) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.window.size()),
                Err(wgpu::SurfaceError::OutOfMemory) => *flow = ControlFlow::Exit,
                Err(e) => eprintln!("{e:?}"),
            };
        }
        Event::MainEventsCleared => {
            state.window.winit.request_redraw();
        }
        _ => {}
    });
}
