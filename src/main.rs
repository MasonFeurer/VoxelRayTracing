#![allow(dead_code)]
#![feature(let_chains)]

pub mod aabb;
pub mod cam;
pub mod input;
pub mod matrices;
pub mod open_simplex;
pub mod player;
pub mod shader;
pub mod vectors;
pub mod world;

use crate::cam::HitResult;
use crate::input::{InputState, Key};
use crate::player::Player;
use crate::shader::{Settings, Shader};
use crate::vectors::{Vec2, Vec3};
use crate::world::{Voxel, World};
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window as WinitWindow;
use winit::window::{CursorGrabMode, Fullscreen, WindowBuilder};

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
}
impl Gpu {
    fn create_shader(&self, buffer_size: Vec2<u32>) -> Shader {
        Shader::new(&self.device, &self.surface_config, buffer_size)
    }

    fn resize(&mut self, new_size: Vec2<u32>) {
        self.surface_config.width = new_size.x;
        self.surface_config.height = new_size.y;
        self.surface.configure(&self.device, &self.surface_config);
    }
}
async fn init_wgpu(window: &WinitWindow) -> Gpu {
    let size = window.inner_size();
    let size = Vec2::new(size.width, size.height);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    // Handle to a presentable surface
    let surface = unsafe { instance.create_surface(window) }.unwrap();

    // Handle to the graphics device
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    // device: Open connection to graphics device
    // queue: Handle to a command queue on the device
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        )
        .await
        .unwrap();

    let surface_config = surface
        .get_default_config(&adapter, size.x, size.y)
        .unwrap();
    surface.configure(&device, &surface_config);

    Gpu {
        surface,
        device,
        surface_config,
        queue,
    }
}

struct Egui {
    wgpu: egui_wgpu::renderer::Renderer,
    winit: egui_winit::State,
    ctx: egui::Context,
}
impl Egui {
    fn new(event_loop: &EventLoop<()>, gpu: &Gpu) -> Self {
        Self {
            wgpu: egui_wgpu::renderer::Renderer::new(
                &gpu.device,
                gpu.surface_config.format,
                None,
                1,
            ),
            winit: egui_winit::State::new(&event_loop),
            ctx: egui::Context::default(),
        }
    }
}

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

    fn size(&self) -> Vec2<u32> {
        <[u32; 2]>::from(self.winit.inner_size()).into()
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
        if locked {
            self.winit.set_cursor_grab(CursorGrabMode::Locked).unwrap();
            self.winit.set_cursor_visible(false);
        } else {
            self.winit.set_cursor_grab(CursorGrabMode::None).unwrap();
            self.winit.set_cursor_visible(true);
        }
        self.cursor_locked = locked;
    }
    fn toggle_cursor_locked(&mut self) {
        self.set_cursor_locked(!self.cursor_locked)
    }
}

struct State {
    window: Window,
    gpu: Gpu,
    shader: Shader,
    settings: Settings,

    player: Player,
    hit_result: Option<HitResult>,
    world: Box<World>,
}
impl State {
    fn new(window: Window, gpu: Gpu) -> Self {
        let settings = Settings::default();

        let mut world = Box::new(World::new());
        world.populate();
        let player = Player::new(Vec3::new(50.0, 100.0, 50.0));

        // Create shader
        let shader = gpu.create_shader(window.size());
        shader.world_buffer.update(&gpu.queue, &world);
        shader.settings_buffer.update(&gpu.queue, settings);

        Self {
            window,
            gpu,
            shader,
            settings,

            player,
            hit_result: None,
            world,
        }
    }

    fn update(&mut self, input: &InputState) {
        if self.window.cursor_locked {
            self.player.update(1.0, input, &self.world);
            self.shader
                .cam_buffer
                .update(&self.gpu.queue, &self.player.cam());
        }
        if input.key_pressed(Key::T) {
            self.window.toggle_cursor_locked();
        }
        if input.key_pressed(Key::F) {
            self.window.toggle_fullscreen();
        }
        self.shader.rand_floats_buffer.update(&self.gpu.queue);

        self.hit_result = self.player.cast_ray(&self.world);

        if let Some(hit) = self.hit_result && input.left_button_pressed() {
            let (chunk_idx, _) = self.world.set_voxel(hit.pos.unsigned().unwrap(), Voxel::AIR).unwrap();
            self.shader.world_buffer.update_chunk(&self.gpu.queue, chunk_idx, self.world.chunks[chunk_idx]);
        }
        if let Some(hit) = self.hit_result && input.right_button_pressed() {
            let Some(place_pos) = (hit.pos + hit.face).unsigned() else {
                return;
            };
            let (chunk_idx, _) = self.world.set_voxel(place_pos, Voxel::IRON).unwrap();
            self.shader.world_buffer.update_chunk(&self.gpu.queue, chunk_idx, self.world.chunks[chunk_idx]);
        }
    }

    fn resize(&mut self, new_size: Vec2<u32>) {
        self.gpu.resize(new_size);
        self.shader.proj_buffer.update(
            &self.gpu.queue,
            self.shader.color_buffer.size(),
            &self.player,
        );
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        fn value(ui: &mut egui::Ui, label: &str, v: &mut f32, min: f32, max: f32) -> bool {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add(egui::Slider::new(v, min..=max)).changed()
            })
            .inner
        }
        fn color(ui: &mut egui::Ui, label: &str, color: &mut [f32; 4]) -> bool {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.color_edit_button_rgba_premultiplied(color).changed()
            })
            .inner
        }

        let Settings {
            ray_dist,
            water_color,
            min_water_opacity,
            water_opacity_max_dist,
            sky_color,
            ..
        } = &mut self.settings;

        let mut changed = false;
        changed |= value(ui, "ray dist: ", ray_dist, 0.0, 300.0);
        changed |= value(ui, "min water opacity: ", min_water_opacity, 0.0, 1.0);
        changed |= value(
            ui,
            "water opacity max dist: ",
            water_opacity_max_dist,
            0.0,
            100.0,
        );
        changed |= color(ui, "water color", water_color);
        changed |= color(ui, "sky color", sky_color);

        if changed {
            self.shader
                .settings_buffer
                .update(&self.gpu.queue, self.settings);
        }
    }

    fn render(&mut self, egui: &mut Egui) -> Result<(), wgpu::SurfaceError> {
        let egui_input = egui.winit.take_egui_input(&self.window.winit);
        let egui_output = egui.ctx.run(egui_input, |ctx| {
            egui::TopBottomPanel::top("top").show(&ctx, |ui| {
                self.settings_ui(ui);
            });
        });

        let egui_prims = egui.ctx.tessellate(egui_output.shapes);
        let screen_desc = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: self.window.size().into(),
            pixels_per_point: egui_winit::native_pixels_per_point(&self.window.winit),
        };

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

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("#compute_pass"),
        });
        compute_pass.set_pipeline(&self.shader.compute_pipeline);
        compute_pass.set_bind_group(0, &self.shader.compute_bind_group, &[]);
        let [buffer_w, buffer_h]: [u32; 2] = self.shader.color_buffer.size().into();
        compute_pass.dispatch_workgroups(buffer_w, buffer_h, 1);
        std::mem::drop(compute_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("#render_pass"),
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
        render_pass.set_pipeline(&self.shader.render_pipeline);
        render_pass.set_bind_group(0, &self.shader.render_bind_group, &[]);
        render_pass.draw(0..6, 0..1);
        std::mem::drop(render_pass);

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

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        egui.wgpu
            .render(&mut render_pass, &egui_prims, &screen_desc);
        std::mem::drop(render_pass);

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        for id in egui_output.textures_delta.free {
            egui.wgpu.free_texture(&id);
        }

        Ok(())
    }
}

pub fn main() {
    use std::time::SystemTime;
    env_logger::init();

    let event_loop = EventLoop::new();

    let mut input = input::InputState::default();
    let mut window = Window::new(&event_loop);
    window.set_cursor_locked(true);

    let gpu = pollster::block_on(init_wgpu(&window.winit));

    let mut egui = Egui::new(&event_loop, &gpu);
    egui.winit
        .set_pixels_per_point(egui_winit::native_pixels_per_point(&window.winit));
    let mut state = State::new(window, gpu);
    let mut last_frame = SystemTime::now();

    event_loop.run(move |event, _, control_flow| match event {
        event if input.update(&event) => {}
        Event::WindowEvent { event, .. } => match event {
            e if egui.winit.on_event(&egui.ctx, &e).consumed => {}
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(size) => {
                state.resize(Vec2::new(size.width, size.height));
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                let size = *new_inner_size;
                state.resize(Vec2::new(size.width, size.height));
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

            let title = format!(
                "{} {} {}",
                state.player.pos.x, state.player.pos.y, state.player.pos.z
            );
            state.window.winit.set_title(&title);
            state.update(&input);

            input.finish_frame();
            match state.render(&mut egui) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.window.size()),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{e:?}"),
            };
        }
        Event::MainEventsCleared => {
            state.window.winit.request_redraw();
        }
        _ => {}
    });
}
