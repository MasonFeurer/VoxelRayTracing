pub mod egui;
pub mod texture;

use crate::world::{ChunkHeader, Node, World};
use glam::{uvec2, Mat4, UVec2, Vec2, Vec3};
use texture::Texture;

use wgpu::*;

#[derive(Clone)]
#[repr(C)]
pub struct Material {
    pub color: [f32; 3],
    pub empty: u32,
    pub scatter: f32,
    pub emission: f32,
    pub polish_bounce_chance: f32,
    pub translucency: f32,
    pub polish_color: [f32; 3],
    pub polish_scatter: f32,
}

static RAY_TRACER_SRC: &str = include_str!("ray_tracer.wgsl");
static PATH_TRACER_SRC: &str = include_str!("path_tracer.wgsl");
static SCREEN_SHADER_SRC: &str = include_str!("screen_shader.wgsl");

const RESULT_TEX_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
const RESULT_TEX_USAGES: TextureUsages = TextureUsages::COPY_DST
    .union(TextureUsages::COPY_SRC)
    .union(TextureUsages::STORAGE_BINDING)
    .union(TextureUsages::TEXTURE_BINDING);

pub struct SimpleBuffer<T>(pub Buffer, std::marker::PhantomData<T>);
impl<T> SimpleBuffer<T> {
    pub fn new(gpu: &Gpu, label: &str, usage: BufferUsages) -> Self {
        let buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size: std::mem::size_of::<T>() as u64,
            usage,
            mapped_at_creation: false,
        });
        Self(buffer, std::marker::PhantomData)
    }

    pub fn write(&self, gpu: &Gpu, value: &T) {
        let ptr = value as *const T as *const u8;
        let size = std::mem::size_of::<T>();
        let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
        gpu.queue.write_buffer(&self.0, 0, slice);
    }
}
impl<T, const N: usize> SimpleBuffer<[T; N]> {
    pub fn write_slice(&self, gpu: &Gpu, idx: u64, slice: &[T]) {
        let ptr = slice.as_ptr() as *const u8;
        let size = std::mem::size_of::<T>() * slice.len();
        let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
        gpu.queue.write_buffer(&self.0, idx, slice);
    }
}

pub struct ArrayBuffer<T>(Buffer, u32, std::marker::PhantomData<T>);
impl<T> ArrayBuffer<T> {
    pub fn new(gpu: &Gpu, label: &str, usage: BufferUsages, size: u32) -> Self {
        let handle = gpu.device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size: size as u64 * std::mem::size_of::<T>() as u64,
            usage,
            mapped_at_creation: false,
        });
        Self(handle, size, std::marker::PhantomData)
    }

    pub fn write(&self, gpu: &Gpu, offset: u64, items: &[T]) {
        let items_cut = (items.len() as u64).min(self.1 as u64 - offset);
        let items: &[T] = &items[0..items_cut as usize];

        let ptr = items.as_ptr() as *const u8;
        let size = items.len() * std::mem::size_of::<T>();
        let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
        let offset = offset * std::mem::size_of::<T>() as u64;
        gpu.queue.write_buffer(&self.0, offset, slice);
    }
}

fn uniform_binding_type() -> BindingType {
    BindingType::Buffer {
        ty: BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    }
}
fn storage_binding_type(read_only: bool) -> BindingType {
    BindingType::Buffer {
        ty: BufferBindingType::Storage { read_only },
        has_dynamic_offset: false,
        min_binding_size: None,
    }
}

macro_rules! bind_group_layout_entries {
    ($($binding:expr=>($vis:ident)$entry:expr),*$(,)?) => {{[
        $(BindGroupLayoutEntry {
            binding: $binding,
            visibility: ShaderStages::$vis,
            ty: $entry,
            count: None,
        }),*
    ]}}
}
macro_rules! bind_group_entries {
    ($($binding:expr=>$entry:expr),*$(,)?) => {{[
        $(BindGroupEntry {
            binding: $binding,
            resource: $entry,
        }),*
    ]}}
}

pub struct ScreenShader {
    pub pipeline: RenderPipeline,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
}
impl ScreenShader {
    pub fn new(gpu: &Gpu, tex: &Texture, surface_format: TextureFormat) -> Self {
        let device = &gpu.device;
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("screen-shader.shader-module"),
            source: ShaderSource::Wgsl(SCREEN_SHADER_SRC.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("screen-shader.bind-group-layout"),
            entries: &bind_group_layout_entries!(
                0 => (FRAGMENT) BindingType::Texture {
                    sample_type: TextureSampleType::default(),
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                1 => (FRAGMENT) BindingType::Sampler(SamplerBindingType::Filtering),
            ),
        });
        let bind_group = Self::create_bind_group(gpu, &bind_group_layout, tex);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("screen-shader.pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("screen-shader.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn create_bind_group(gpu: &Gpu, layout: &BindGroupLayout, tex: &Texture) -> BindGroup {
        gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("screen-shader.bind_group"),
            layout,
            entries: &bind_group_entries!(
                0 => BindingResource::TextureView(&tex.view),
                1 => BindingResource::Sampler(&tex.sampler),
            ),
        })
    }

    pub fn recreate_bind_group(&mut self, gpu: &Gpu, tex: &Texture) {
        self.bind_group = Self::create_bind_group(gpu, &self.bind_group_layout, tex);
    }

    pub fn encode_pass(&self, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("#screen-shader-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}

pub struct PixelShader {
    pub pipeline: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
}
impl PixelShader {
    pub fn new(src: &str, gpu: &Gpu, tex: &Texture, buffers: &Buffers) -> Self {
        let device = &gpu.device;
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("#pixel-shader.shader-module"),
            source: ShaderSource::Wgsl(src.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("#pixel-shader.bind-group-layout"),
            entries: &bind_group_layout_entries!(
                0 => (COMPUTE) BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: RESULT_TEX_FORMAT,
                    view_dimension: TextureViewDimension::D2,
                },
                1 => (COMPUTE) uniform_binding_type(),
                2 => (COMPUTE) uniform_binding_type(),
                3 => (COMPUTE) storage_binding_type(true),
                5 => (COMPUTE) uniform_binding_type(),
                6 => (COMPUTE) storage_binding_type(true),
                7 => (COMPUTE) storage_binding_type(true),
            ),
        });
        let bind_group = Self::create_bind_group(gpu, &bind_group_layout, tex, buffers);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("#pixel-shader.pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("#pixel-shader.pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("update"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn create_bind_group(
        gpu: &Gpu,
        layout: &BindGroupLayout,
        output_tex: &Texture,
        buffers: &Buffers,
    ) -> BindGroup {
        gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("#raytracer.bind-broup"),
            layout,
            entries: &bind_group_entries!(
                0 => BindingResource::TextureView(&output_tex.view),
                1 => buffers.cam_data.0.as_entire_binding(),
                2 => buffers.settings.0.as_entire_binding(),
                3 => buffers.voxel_materials.0.as_entire_binding(),
                5 => buffers.world_data.0.as_entire_binding(),
                6 => buffers.nodes.0.as_entire_binding(),
                7 => buffers.chunks.0.as_entire_binding(),
            ),
        })
    }

    pub fn recreate_bind_group(&mut self, gpu: &Gpu, tex: &Texture, buffers: &Buffers) {
        self.bind_group = Self::create_bind_group(gpu, &self.bind_group_layout, tex, buffers);
    }

    pub fn encode_pass(&self, encoder: &mut CommandEncoder, workgroups: UVec2) {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("#raytracer-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(workgroups.x, workgroups.y, 1);
    }
}

pub struct Buffers {
    pub cam_data: SimpleBuffer<CamData>,
    pub settings: SimpleBuffer<Settings>,
    pub world_data: SimpleBuffer<WorldData>,
    pub nodes: ArrayBuffer<Node>,
    pub voxel_materials: SimpleBuffer<[Material; 256]>,
    pub chunks: ArrayBuffer<ChunkHeader>,
}
impl Buffers {
    pub fn new(gpu: &Gpu, max_nodes: u32, world_size: u32) -> Self {
        const COPY_DST: BufferUsages = BufferUsages::COPY_DST;
        const UNIFORM: BufferUsages = BufferUsages::UNIFORM;
        const STORAGE: BufferUsages = BufferUsages::STORAGE;
        let chunk_count = world_size * world_size * world_size;

        Self {
            cam_data: SimpleBuffer::new(gpu, "cam_data", COPY_DST | UNIFORM),
            settings: SimpleBuffer::new(gpu, "settings", COPY_DST | UNIFORM),
            world_data: SimpleBuffer::new(gpu, "world_data", COPY_DST | UNIFORM),
            nodes: ArrayBuffer::new(gpu, "nodes", COPY_DST | STORAGE, max_nodes),
            voxel_materials: SimpleBuffer::new(gpu, "voxel_mats", COPY_DST | STORAGE),
            chunks: ArrayBuffer::new(gpu, "chunks", COPY_DST | STORAGE, chunk_count),
        }
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct CamData {
    pub pos: Vec3,
    pub _padding0: u32,
    pub inv_view_mat: Mat4,
    pub inv_proj_mat: Mat4,
    pub proj_size: Vec2,
    pub _padding1: [u32; 2],
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct WorldData {
    pub min: [i32; 3],
    pub size: u32,
    pub size_in_chunks: u32,
    _padding: [u32; 3],
}
impl WorldData {
    pub fn from(world: &World) -> Self {
        Self {
            min: world.min().into(),
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
    pub samples_per_pixel: u32,
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

        let screen_shader = ScreenShader::new(gpu, &result_texture, surface_format);
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

    pub fn resize_result_texture(&mut self, gpu: &Gpu, new_size: UVec2) {
        self.result_texture =
            Texture::new(&gpu.device, new_size, RESULT_TEX_FORMAT, RESULT_TEX_USAGES);

        self.screen_shader
            .recreate_bind_group(gpu, &self.result_texture);

        self.ray_tracer
            .recreate_bind_group(gpu, &self.result_texture, &self.buffers);
        self.path_tracer
            .recreate_bind_group(gpu, &self.result_texture, &self.buffers);
    }
}

pub struct Gpu<'a> {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'a>,
    pub surface_config: SurfaceConfiguration,
}
impl<'a> Gpu<'a> {
    pub async fn new(window: &'a winit::window::Window) -> Self {
        let size = window.inner_size();
        let size = uvec2(size.width, size.height);

        let instance = Instance::new(&Default::default());
        // Handle to a presentable surface
        let surface = unsafe { instance.create_surface(window) }.unwrap();

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
