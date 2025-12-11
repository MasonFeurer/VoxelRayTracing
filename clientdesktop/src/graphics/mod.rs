pub mod egui;
pub mod shader;
pub mod texture;

pub use egui::Egui;

use client::common::world::NodeAddr;
use client::world::ClientWorld;
use glam::{uvec2, vec2, Mat4, UVec2, Vec2, Vec3};
use shader::*;
use std::sync::Arc;
use texture::Texture;
use wgpu::*;

static RAY_TRACER_SRC: &str = include_str!("ray_tracer.wgsl");
static PATH_TRACER_SRC: &str = include_str!("path_tracer.wgsl");
static SCREEN_SHADER_SRC: &str = include_str!("screen_shader.wgsl");

#[derive(Clone)]
#[repr(C)]
pub struct Material {
    pub color: [f32; 3],
    pub is_empty: u32,
    pub is_liquid: u32,
    pub scatter: f32,
    _padding: [u32; 2],
}
impl Material {
    pub fn construct(src: client::common::resources::VoxelStyle) -> Self {
        use client::common::resources::VoxelState;
        Self {
            color: src.color,
            is_empty: (src.state == VoxelState::Gas) as u32,
            is_liquid: (src.state == VoxelState::Liquid) as u32,
            scatter: 0.0,
            _padding: [0; 2],
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct Crosshair {
    pub color: [f32; 4],
    pub style: u32,
    pub size: f32,
    _padding: [u32; 2],
}
impl Default for Crosshair {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 0.33],
            style: 2,
            size: 5.0,
            _padding: [0; 2],
        }
    }
}

#[derive(Clone, Default)]
#[repr(C)]
pub struct CamData {
    pub pos: Vec3,
    pub _padding0: u32,
    pub inv_view_mat: Mat4,
    pub inv_proj_mat: Mat4,
    pub proj_size: Vec2,
    pub _padding1: [u32; 2],
}
impl CamData {
    pub fn create(cam: Vec3, eye: Vec3, fov: f32, proj_size: Vec2) -> Self {
        let inv_view_mat = Mat4::from_translation(eye)
            * Mat4::from_rotation_x(cam.x.to_radians())
            * Mat4::from_rotation_y(-cam.y.to_radians())
            * Mat4::from_rotation_z(cam.z.to_radians());
        let inv_proj_mat =
            Mat4::perspective_rh(fov.to_radians(), proj_size.x / proj_size.y, 0.001, 1000.0)
                .inverse();

        CamData {
            pos: eye,
            inv_view_mat,
            inv_proj_mat,
            proj_size: vec2(proj_size.x, proj_size.y),
            _padding0: 0,
            _padding1: [0; 2],
        }
    }
}

#[derive(Clone, Default)]
#[repr(C)]
pub struct WorldData {
    pub min: [i32; 3],
    pub size: u32,
    pub size_in_chunks: u32,
    _padding: [u32; 3],
}
impl WorldData {
    pub fn from(world: &ClientWorld) -> Self {
        Self {
            min: world.min_voxel().into(),
            size: world.size(),
            size_in_chunks: world.size_in_chunks(),
            _padding: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct Settings {
    pub max_ray_bounces: u32,
    pub sun_intensity: f32,
    pub show_step_count: u32,
    _padding0: u32,
    pub sky_color: [f32; 3],
    pub _padding1: u32,
    pub sun_pos: [f32; 3],
    pub _padding2: u32,
}

pub struct GpuResources {
    pub result_texture: Texture,
    pub voxel_texture_atlas: Texture,
    pub buffers: Buffers,

    pub screen_shader: ScreenShader,
    pub ray_tracer: PixelShader,
    pub path_tracer: PixelShader,
}
impl GpuResources {
    pub fn new(
        gpu: &Gpu,
        surface_format: TextureFormat,
        result_size: UVec2,
        max_nodes: u32,
        world_size: u32,
    ) -> Self {
        let buffers = Buffers::new(gpu, max_nodes, world_size);

        let result_texture = Texture::new(
            &gpu.device,
            result_size,
            RESULT_TEX_FORMAT,
            RESULT_TEX_USAGES,
        );
        let voxel_texture_atlas = Texture::new(
            &gpu.device,
            uvec2(5, 5),
            TextureFormat::Rgba8Unorm,
            TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        );

        let screen_shader = ScreenShader::new(
            SCREEN_SHADER_SRC,
            gpu,
            &result_texture,
            surface_format,
            &buffers,
        );
        let ray_tracer = PixelShader::new(RAY_TRACER_SRC, gpu, &result_texture, &buffers);
        let path_tracer = PixelShader::new(PATH_TRACER_SRC, gpu, &result_texture, &buffers);

        Self {
            result_texture,
            voxel_texture_atlas,
            buffers,
            screen_shader,
            ray_tracer,
            path_tracer,
        }
    }

    pub fn use_new_world_size(&mut self, gpu: &Gpu, world_size: u32) {
        self.buffers.resize_chunk_buffer(gpu, world_size);
    }

    pub fn resize_result_texture(&mut self, gpu: &Gpu, new_size: UVec2) {
        self.result_texture =
            Texture::new(&gpu.device, new_size, RESULT_TEX_FORMAT, RESULT_TEX_USAGES);

        self.screen_shader
            .recreate_bind_group(gpu, &self.result_texture, &self.buffers);
        self.ray_tracer
            .recreate_bind_group(gpu, &self.result_texture, &self.buffers);
        self.path_tracer
            .recreate_bind_group(gpu, &self.result_texture, &self.buffers);
    }
}

pub async fn gpu_limits() -> Limits {
    let instance = Instance::new(&Default::default());
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: Default::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    adapter.limits()
}

pub struct Gpu {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
}
impl Gpu {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();
        let size = uvec2(size.width, size.height);

        let instance = Instance::new(&Default::default());
        // Handle to a presentable surface
        let surface = instance.create_surface(window).unwrap();

        // Handle to the graphics device
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let max_storage_buffer_binding_size = adapter.limits().max_storage_buffer_binding_size;
        let max_buffer_size = adapter.limits().max_buffer_size;

        // device: Open connection to graphics device
        // queue: Handle to a command queue on the device
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                required_features: Features::default(),
                required_limits: Limits {
                    max_storage_buffer_binding_size,
                    max_buffer_size,
                    ..Default::default()
                },
                label: None,
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .unwrap();

        let surface_config = surface
            .get_default_config(&adapter, size.x, size.y)
            .unwrap();
        surface.configure(&device, &surface_config);

        Self {
            surface,
            device,
            surface_config,
            queue,
        }
    }

    pub fn resize(&mut self, new_size: UVec2) {
        self.surface_config.width = new_size.x;
        self.surface_config.height = new_size.y;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn create_command_encoder(&self) -> CommandEncoder {
        self.device.create_command_encoder(&Default::default())
    }

    pub fn surface_size(&self) -> UVec2 {
        uvec2(self.surface_config.width, self.surface_config.height)
    }

    pub fn get_output(&self) -> Result<(SurfaceTexture, TextureView), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        Ok((output, view))
    }
}
