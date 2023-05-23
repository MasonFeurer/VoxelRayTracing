#![feature(new_uninit)]
#![feature(let_chains)]

pub mod debug;
pub mod gpu;
pub mod input;
pub mod math;
pub mod player;
pub mod utils;
pub mod world;

// Intersesting WorldGen settings:
// scale: 5.4
// freq: 1.3

use crate::gpu::shaders::{Settings, Shaders};
use crate::gpu::{debug::Egui, Gpu};
use crate::input::{InputState, Key};
use crate::math::dda::HitResult;
use crate::player::Player;
use crate::world::{DefaultWorldGen, Material, Voxel, World};
use glam::{UVec2, Vec3};
use std::time::SystemTime;
use winit::event_loop::EventLoop;
use winit::window::Window as WinitWindow;
use winit::window::{CursorGrabMode, Fullscreen, WindowBuilder};
use world::DEFAULT_VOXEL_MATERIALS;

pub struct Window {
    pub winit: WinitWindow,
    pub cursor_locked: bool,
}
impl Window {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let winit = WindowBuilder::new()
            .with_title("Voxel Ray Tracing")
            .build(&event_loop)
            .unwrap();

        Self {
            winit,
            cursor_locked: false,
        }
    }

    pub fn size(&self) -> UVec2 {
        <[u32; 2]>::from(self.winit.inner_size()).into()
    }

    pub fn aspect(&self) -> f32 {
        self.size().x as f32 / self.size().y as f32
    }

    pub fn toggle_fullscreen(&mut self) {
        self.winit.set_fullscreen(match self.winit.fullscreen() {
            Some(_) => None,
            None => Some(Fullscreen::Borderless(None)),
        });
    }
    pub fn set_cursor_locked(&mut self, locked: bool) {
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
    pub fn toggle_cursor_locked(&mut self) {
        self.set_cursor_locked(!self.cursor_locked)
    }
}

pub static INVENTORY: &[Voxel] = &[
    Voxel(Voxel::STONE),
    Voxel(Voxel::DIRT),
    Voxel(Voxel::GRASS),
    Voxel(Voxel::FIRE),
    Voxel(Voxel::MAGMA),
    Voxel(Voxel::WATER),
    Voxel(Voxel::WOOD),
    Voxel(Voxel::BARK),
    Voxel(Voxel::LEAVES),
    Voxel(Voxel::SAND),
    Voxel(Voxel::MUD),
    Voxel(Voxel::CLAY),
    Voxel(Voxel::GOLD),
    Voxel(Voxel::MIRROR),
    Voxel(Voxel::BRIGHT),
    Voxel(Voxel::ORANGE_TILE),
    Voxel(Voxel::POLISHED_BLACK_TILES),
    Voxel(Voxel::SMOOTH_ROCK),
    Voxel(Voxel::WOOD_FLOORING),
    Voxel(Voxel::POLISHED_BLACK_FLOORING),
];

pub struct State {
    pub window: Window,
    pub gpu: Gpu,
    pub shaders: Shaders,
    pub settings: Settings,

    pub player: Player,
    pub hit_result: Option<HitResult>,
    pub world: Box<World>,
    pub inv_sel: u8,
    pub last_second: SystemTime,
    pub fps: u32,
    pub fps_temp: u32,
    pub world_depth: u32,

    pub resize_output_tex: bool,
    pub output_tex_h: u32,

    pub world_gen: DefaultWorldGen,
    pub sun_angle: f32,
    pub frame_count: u32,
    pub voxel_materials: Vec<Material>,
}
impl State {
    pub fn new(window: Window, gpu: Gpu) -> Self {
        let mut settings = Settings::default();
        settings.samples_per_pixel = 1;
        settings.max_ray_bounces = 3;
        settings.max_ray_steps = 100;
        settings.sun_intensity = 4.0;
        settings.sky_color = [0.81, 0.93, 1.0];

        let world_depth = 7;

        let mut world = World::new_boxed(world_depth);

        let world_gen = DefaultWorldGen::new(fastrand::i64(..), 1.0, 1.0);
        _ = world.populate_with(&world_gen);

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
        let voxel_materials = DEFAULT_VOXEL_MATERIALS.to_vec();
        shaders
            .raytracer
            .voxel_materials
            .write_slice(&gpu.queue, 0, &voxel_materials);
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
            inv_sel: 19,
            last_second: SystemTime::now(),
            fps: 0,
            fps_temp: 0,
            world_depth,

            output_tex_h,
            resize_output_tex: false,
            sun_angle: 0.0,
            frame_count: 0,
            voxel_materials,
        }
    }

    pub fn update(&mut self, input: &InputState) {
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

        if input.key_pressed(Key::Right) && self.inv_sel < 19 {
            self.inv_sel += 1;
        }
        if input.key_pressed(Key::Left) && self.inv_sel > 0 {
            self.inv_sel -= 1;
        }

        if !self.window.cursor_locked {
            return;
        }

        if let Some(hit) = self.hit_result && input.left_button_pressed() {
            if let Ok(()) = self.world.set_voxel(hit.pos, Voxel(Voxel::AIR)) {
                self.shaders.raytracer.world.write(&self.gpu.queue, &self.world);
                self.shaders.resize_output_tex(&self.gpu.device, self.shaders.output_texture.size());
                self.frame_count = 0;
            } else {
                println!("failed to set voxel");
            }
        }
        if let Some(hit) = self.hit_result && input.right_button_pressed() {
            let voxel_in_hand = INVENTORY[self.inv_sel as usize];
            if let Ok(()) = self.world.set_voxel(hit.pos + hit.face, voxel_in_hand) {
                self.shaders.raytracer.world.write(&self.gpu.queue, &self.world);
                self.shaders.resize_output_tex(&self.gpu.device, self.shaders.output_texture.size());
                self.frame_count = 0;
            } else {
                println!("failed to set voxel");
            }
        }
    }

    pub fn render(&mut self, egui: &mut Egui) -> Result<(), wgpu::SurfaceError> {
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
                    crate::debug::debug_ui(self, ui);
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

    pub fn resize(&mut self, new_size: UVec2) {
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
}
