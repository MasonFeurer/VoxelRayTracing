pub mod egui;
pub mod texture;

use crate::world::{Material, World};
use glam::{Mat4, UVec2, Vec2, Vec3};
use texture::Texture;

use wgpu::*;

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
            label: Some("output-tex-shader.shader-module"),
            source: ShaderSource::Wgsl(SCREEN_SHADER_SRC.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("output-tex-shader.bind-group-layout"),
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
            label: Some("output-tex-shader.pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("output-tex-shader.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        Self {
            pipeline,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn create_bind_group(gpu: &Gpu, layout: &BindGroupLayout, tex: &Texture) -> BindGroup {
        gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("output-tex-shader.bind_group"),
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
            label: Some("#output-tex-shader-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
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
            label: Some("#raytracer.shader-module"),
            source: ShaderSource::Wgsl(src.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("#raytracer.bind-group-layout"),
            entries: &bind_group_layout_entries!(
                0 => (COMPUTE) BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: RESULT_TEX_FORMAT,
                    view_dimension: TextureViewDimension::D2,
                },
                1 => (COMPUTE) uniform_binding_type(),
                2 => (COMPUTE) uniform_binding_type(),
                3 => (COMPUTE) storage_binding_type(true),
                4 => (COMPUTE) storage_binding_type(true),
                5 => (COMPUTE) uniform_binding_type(),
            ),
        });
        let bind_group = Self::create_bind_group(gpu, &bind_group_layout, tex, buffers);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("#raytracer.pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("#raytracer.pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "update",
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
                3 => buffers.world.0.as_entire_binding(),
                4 => buffers.voxel_materials.0.as_entire_binding(),
                5 => buffers.frame_count.0.as_entire_binding(),
            ),
        })
    }

    pub fn recreate_bind_group(&mut self, gpu: &Gpu, tex: &Texture, buffers: &Buffers) {
        self.bind_group = Self::create_bind_group(gpu, &self.bind_group_layout, tex, buffers);
    }

    pub fn encode_pass(&self, encoder: &mut CommandEncoder, workgroups: UVec2) {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("#raytracer-pass"),
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(workgroups.x, workgroups.y, 1);
    }
}

pub struct Buffers {
    pub cam_data: SimpleBuffer<CamData>,
    pub settings: SimpleBuffer<Settings>,
    pub world: SimpleBuffer<World>,
    pub voxel_materials: SimpleBuffer<[Material; 256]>,
    pub frame_count: SimpleBuffer<u32>,
}
impl Buffers {
    pub fn new(gpu: &Gpu) -> Self {
        const COPY_DST: BufferUsages = BufferUsages::COPY_DST;
        const UNIFORM: BufferUsages = BufferUsages::UNIFORM;
        const STORAGE: BufferUsages = BufferUsages::STORAGE;

        Self {
            cam_data: SimpleBuffer::new(gpu, "", COPY_DST | UNIFORM),
            settings: SimpleBuffer::new(gpu, "", COPY_DST | UNIFORM),
            world: SimpleBuffer::new(gpu, "", COPY_DST | STORAGE),
            voxel_materials: SimpleBuffer::new(gpu, "", COPY_DST | STORAGE),
            frame_count: SimpleBuffer::new(gpu, "", COPY_DST | UNIFORM),
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
pub struct Settings {
    pub max_ray_bounces: u32,
    pub sun_intensity: f32,
    _padding0: [u32; 2],
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
    pub fn new(gpu: &Gpu, surface_format: TextureFormat, result_size: UVec2) -> Self {
        let buffers = Buffers::new(gpu);

        let result_texture = Texture::new(
            &gpu.device,
            result_size,
            RESULT_TEX_FORMAT,
            RESULT_TEX_USAGES,
        );
        let voxel_texture_atlas = Texture::new(
            &gpu.device,
            UVec2::new(5, 5),
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

pub struct Gpu {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub surface_config: SurfaceConfiguration,
}
impl Gpu {
    pub async fn new(window: &winit::window::Window, max_buffer_sizes: u32) -> Self {
        let size = window.inner_size();
        let size = UVec2::new(size.width, size.height);

        let instance = Instance::new(Default::default());
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

        // device: Open connection to graphics device
        // queue: Handle to a command queue on the device
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::default(),
                    limits: Limits {
                        max_storage_buffer_binding_size: max_buffer_sizes,
                        max_buffer_size: max_buffer_sizes as u64,
                        ..Default::default()
                    },
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
        UVec2::new(self.surface_config.width, self.surface_config.height)
    }

    pub fn get_output(&self) -> Result<(SurfaceTexture, TextureView), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        Ok((output, view))
    }
}

pub struct GpuMesh {
    pub vertex_buf: Buffer,
    pub index_buf: Buffer,
    pub vertex_count: u32,
    pub index_count: u32,
}
impl GpuMesh {
    pub fn new<V>(gpu: &Gpu, vertices: &[V], indices: &[u32]) -> Self {
        use wgpu::util::{BufferInitDescriptor, DeviceExt};

        let v_slice = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                std::mem::size_of::<V>() * vertices.len(),
            )
        };
        let i_slice =
            unsafe { std::slice::from_raw_parts(indices.as_ptr() as *const u8, 4 * indices.len()) };

        let vertex_buf = gpu.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("#vertex-buf"),
            contents: v_slice,
            usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
        });
        let index_buf = gpu.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("#index-buf"),
            contents: i_slice,
            usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
        });
        Self {
            vertex_buf,
            index_buf,
            vertex_count: vertices.len() as u32,
            index_count: indices.len() as u32,
        }
    }
}
