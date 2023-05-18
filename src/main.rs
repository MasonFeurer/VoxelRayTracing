#![feature(new_uninit)]
#![feature(let_chains)]

pub mod gpu;
pub mod input;
pub mod math;
pub mod player;
pub mod world;

// Intersesting WorldGen settings:
// scale: 5.4
// freq: 1.3

use crate::gpu::shaders::{Settings, Shaders};
use crate::gpu::{debug::Egui, Gpu};
use crate::input::{InputState, Key};
use crate::math::dda::HitResult;
use crate::player::Player;
use crate::world::{voxel, DefaultWorldGen, Voxel, World};
use glam::{UVec2, Vec3};
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

    fn aspect(&self) -> f32 {
        self.size().x as f32 / self.size().y as f32
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
    world_depth: u32,

    resize_output_tex: bool,
    output_tex_h: u32,

    world_gen: DefaultWorldGen,
    sun_angle: f32,
    frame_count: u32,
}
impl State {
    fn new(window: Window, gpu: Gpu) -> Self {
        let mut settings = Settings::default();
        settings.samples_per_pixel = 1;
        settings.max_ray_bounces = 3;
        settings.max_ray_steps = 100;
        settings.sky_color = [0.3, 0.7, 1.0];

        world::load_default_props(unsafe { &mut world::VOXEL_PROPS });

        let mut world = unsafe {
            let world = Box::<World>::new_zeroed();
            world.assume_init()
        };
        let world_depth = 7;
        world.init(world_depth);
        let world_gen = DefaultWorldGen::new(fastrand::i64(..), 1.0, 1.0);
        world.populate_with(&world_gen);

        let aspect = window.aspect();

        let output_tex_h = 600;

        let output_tex_size = UVec2::new((output_tex_h as f32 * aspect) as u32, output_tex_h);

        let player = Player::new(Vec3::new(10.0, 80.0, 10.0), 0.2);

        settings.sun_pos = Vec3::new(
            0.0f32.to_radians().sin() * 500.0,
            0.0f32.to_radians().cos() * 500.0,
            world.size as f32 * 0.5,
        )
        .to_array();

        let shaders = Shaders::new(&gpu.device, gpu.surface_config.format, output_tex_size);
        shaders.raytracer.world.write(&gpu.queue, &world);
        shaders.raytracer.settings.write(&gpu.queue, &settings);
        shaders
            .raytracer
            .voxel_props
            .write(&gpu.queue, unsafe { &world::VOXEL_PROPS });
        shaders.raytracer.frame_count.write(&gpu.queue, &0);

        Self {
            window,
            gpu,
            shaders,
            settings,

            player,
            hit_result: None,
            world,
            world_gen,
            voxel_in_hand: voxel::DIRT,
            last_second: SystemTime::now(),
            fps: 0,
            fps_temp: 0,
            world_depth,

            output_tex_h,
            resize_output_tex: false,
            sun_angle: 0.0,
            frame_count: 0,
        }
    }

    fn update(&mut self, input: &InputState) {
        if self.resize_output_tex {
            self.resize_output_tex = false;

            let aspect = self.window.aspect();
            let size = UVec2::new(
                (self.output_tex_h as f32 * aspect) as u32,
                self.output_tex_h,
            );
            self.shaders.resize_output_tex(&self.gpu.device, size)
        }

        if self.window.cursor_locked {
            let prev_pos = self.player.pos;
            let prev_rot = self.player.rot;
            self.player.update(1.0, input, &self.world);

            if prev_pos != self.player.pos || prev_rot != self.player.rot {
                // self.shaders.output_texture.clear(&self.gpu);
                self.shaders
                    .resize_output_tex(&self.gpu.device, self.shaders.output_texture.size());
                self.frame_count = 0;
            }
        }

        let size = self.shaders.output_texture.size().as_vec2();
        self.shaders
            .raytracer
            .cam_data
            .write(&self.gpu.queue, &self.player.create_cam_data(size));

        if input.key_pressed(Key::T) {
            self.window.toggle_cursor_locked();
        }
        if input.key_pressed(Key::F) {
            self.window.toggle_fullscreen();
        }

        self.hit_result = self.player.cast_ray(&self.world);

        if input.key_pressed(Key::Key1) {
            self.voxel_in_hand = voxel::DIRT;
        }
        if input.key_pressed(Key::Key2) {
            self.voxel_in_hand = voxel::GRASS;
        }
        if input.key_pressed(Key::Key3) {
            self.voxel_in_hand = voxel::STONE;
        }
        if input.key_pressed(Key::Key4) {
            self.voxel_in_hand = voxel::GOLD;
        }
        if input.key_pressed(Key::Key5) {
            // self.voxel_in_hand = voxel::MIRROR;
            self.voxel_in_hand = voxel::BRIGHT;
        }
        if input.key_pressed(Key::Key6) {
            self.voxel_in_hand = voxel::WATER;
        }
        if input.key_pressed(Key::Key7) {
            self.voxel_in_hand = voxel::MAGMA;
        }
        if input.key_pressed(Key::Key8) {
            self.voxel_in_hand = voxel::BARK;
        }
        if input.key_pressed(Key::Key9) {
            self.voxel_in_hand = voxel::MUD;
        }
        if input.key_pressed(Key::Key0) {
            self.voxel_in_hand = voxel::CLAY;
        }

        if !self.window.cursor_locked {
            return;
        }

        if let Some(hit) = self.hit_result && input.left_button_pressed() {
            self.world.set_voxel(hit.pos, voxel::AIR);
            self.shaders.raytracer.world.write(&self.gpu.queue, &self.world);
            self.shaders.resize_output_tex(&self.gpu.device, self.shaders.output_texture.size());
            self.frame_count = 0;
        }
        if let Some(hit) = self.hit_result && input.right_button_pressed() {
            self.world.set_voxel(hit.pos + hit.face, self.voxel_in_hand);
            self.shaders.raytracer.world.write(&self.gpu.queue, &self.world);
            self.shaders.resize_output_tex(&self.gpu.device, self.shaders.output_texture.size());
            self.frame_count = 0;
        }
    }

    fn render(&mut self, egui: &mut Egui) -> Result<(), wgpu::SurfaceError> {
        // --- get surface and create encoder ---
        let output = self.gpu.surface.get_current_texture()?;
        let surface_size = UVec2::new(
            self.gpu.surface_config.width,
            self.gpu.surface_config.height,
        );
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("#encoder"),
            });

        self.frame_count += 1;
        self.shaders
            .raytracer
            .frame_count
            .write(&self.gpu.queue, &self.frame_count);

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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            pass.set_pipeline(&self.shaders.output_tex_shader.pipeline);
            pass.set_bind_group(0, &self.shaders.output_tex_shader.bind_group, &[]);
            pass.draw(0..6, 0..1);
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
                size_in_pixels: surface_size.into(),
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

        // --- copy output_texture to prev_output_texture ---
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.shaders.output_texture.0,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &self.shaders.prev_output_texture.0,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.shaders.output_texture.size().x,
                height: self.shaders.output_texture.size().y,
                depth_or_array_layers: 1,
            },
        );

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

    fn resize(&mut self, new_size: UVec2) {
        let prev_output_aspect = self.shaders.output_texture.aspect();

        self.gpu.resize(new_size);

        self.shaders.raytracer.cam_data.write(
            &self.gpu.queue,
            &self.player.create_cam_data(self.window.size().as_vec2()),
        );

        if prev_output_aspect != self.window.aspect() {
            self.resize_output_tex = true;
        }
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

        let in_hand = self.voxel_in_hand;
        let red = Color32::from_rgb(255, 150, 0);
        let green = Color32::from_rgb(0, 255, 0);
        let blue = Color32::from_rgb(0, 255, 255);
        let white = Color32::WHITE;

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

        value_f32(ui, "speed", &mut self.player.speed, 0.1, 3.0);

        ui.separator();

        ui.collapsing("world", |ui| {
            value_u32(ui, "world depth", &mut self.world_depth, 2, 11);
            value_f32(ui, "terrain scale", &mut self.world_gen.scale, 0.1, 10.0);
            value_f32(ui, "terrain freq", &mut self.world_gen.freq, 0.1, 10.0);

            if ui.button("regenerate").clicked() {
                self.world.init(self.world_depth);
                self.world.populate_with(&self.world_gen);
                self.shaders
                    .raytracer
                    .world
                    .write(&self.gpu.queue, &self.world);
            }
        });

        ui.separator();

        let mut changed = false;
        ui.collapsing("shader", |ui| {
            let Settings {
                samples_per_pixel,
                max_ray_steps,
                max_ray_bounces,
                sky_color,
                sun_pos,
                ..
            } = &mut self.settings;

            changed |= value_u32(ui, "max ray steps", max_ray_steps, 0, 300);
            changed |= value_u32(ui, "max ray bounces", max_ray_bounces, 0, 30);
            changed |= value_u32(ui, "samples/pixel", samples_per_pixel, 1, 30);
            changed |= color_picker(ui, "sky color", sky_color);
            if value_f32(ui, "sun pos", &mut self.sun_angle, 0.0, 360.0) {
                changed = true;
                *sun_pos = Vec3::new(
                    self.sun_angle.to_radians().sin() * 500.0,
                    self.sun_angle.to_radians().cos() * 500.0,
                    self.world.size as f32 * 0.5,
                )
                .to_array();
                self.resize_output_tex = true;
            }
            if value_u32(ui, "vertical samples", &mut self.output_tex_h, 50, 2000) {
                self.resize_output_tex = true;
            }
        });

        if changed {
            self.shaders
                .raytracer
                .settings
                .write(&self.gpu.queue, &self.settings);
        }
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
