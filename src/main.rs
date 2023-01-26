#![allow(dead_code)]

pub mod cam;
pub mod input;
pub mod matrices;
pub mod open_simplex;
pub mod shader;
pub mod vectors;
pub mod world;

use crate::cam::Cam;
use crate::input::InputState;
use crate::shader::Shader;
use crate::vectors::Vec2;
use crate::world::World;
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: Vec2<u32>,
    size_changed: bool,
    shader: Shader,

    cam: Cam,
    world: Box<World>,
}
impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let size = Vec2::new(size.width, size.height);

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        // Handle to a presentable surface
        let surface = unsafe { instance.create_surface(window) };

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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.x,
            height: size.y,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        surface.configure(&device, &config);

        // Create shader
        let shader = Shader::new(&device, &config, size);

        let mut world = Box::new(World::new());
        world.populate();
        let cam = Cam::new();

        shader.world_buffer.update(&queue, &world);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            size_changed: false,
            shader,

            world,
            cam,
        }
    }

    fn resize(&mut self, new_size: Vec2<u32>) {
        if new_size.x > 0 && new_size.y > 0 {
            self.size_changed = true;
            self.size = new_size;
            self.config.width = new_size.x;
            self.config.height = new_size.y;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update(&mut self, input: &InputState) {
        self.cam.update(0.5, input);
        self.shader.cam_buffer.update(&self.queue, &self.cam);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        if self.size_changed {
            self.shader
                .proj_buffer
                .update(&self.queue, self.size, &self.cam);
            self.size_changed = false;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("#encoder"),
            });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("#compute_pass"),
        });
        compute_pass.set_pipeline(&self.shader.compute_pipeline);
        compute_pass.set_bind_group(0, &self.shader.compute_bind_group, &[]);
        compute_pass.dispatch_workgroups(self.size.x, self.size.y, 1);
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

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rocky")
        .build(&event_loop)
        .unwrap();
    window.set_cursor_visible(false);

    let mut state = pollster::block_on(State::new(&window));
    let mut input = input::InputState::default();

    event_loop.run(move |event, _, control_flow| match event {
        event if input.update(&event) => {}
        Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
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
            window.set_title(&format!("{:?}", state.cam.pos));
            state.update(&input);
            input.finish_frame();
            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{e:?}"),
            };
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}
