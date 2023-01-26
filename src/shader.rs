use crate::cam::Cam;
use crate::vectors::Vec2;
use crate::world::{Chunk, World, WORLD_CHUNKS_COUNT};
use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::*;

static COMPUTE_SHADER: &str = include_str!("../res/shader.wgsl");
static SCREEN_SHADER: &str = include_str!("../res/screen_shader.wgsl");

const COLOR_BUFFER_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

pub struct ColorBuffer {
    texture: Texture,
    view: TextureView,
    sampler: Sampler,
}
impl ColorBuffer {
    pub fn new(device: &Device, size: Vec2<u32>) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("color_buffer"),
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: COLOR_BUFFER_FORMAT,
            usage: TextureUsages::COPY_DST
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING,
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 1.0,
            lod_max_clamp: 1.0,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
        });
        Self {
            texture,
            view,
            sampler,
        }
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

pub struct Shader {
    pub render_pipeline: RenderPipeline,
    pub render_bind_group: BindGroup,

    pub compute_pipeline: ComputePipeline,
    pub compute_bind_group: BindGroup,

    pub color_buffer: ColorBuffer,
    pub cam_buffer: CamBuffer,
    pub proj_buffer: ProjBuffer,
    pub world_buffer: WorldBuffer,
}
impl Shader {
    pub fn new(device: &Device, config: &SurfaceConfiguration, size: Vec2<u32>) -> Self {
        let cam_buffer = CamBuffer::new(device);
        let proj_buffer = ProjBuffer::new(device);
        let world_buffer = WorldBuffer::new(device);
        let color_buffer = ColorBuffer::new(device, size);

        let (compute_pipeline, compute_bind_group) = create_compute_pipeline(
            device,
            &color_buffer,
            &cam_buffer,
            &proj_buffer,
            &world_buffer,
        );
        let (render_pipeline, render_bind_group) =
            create_render_pipeline(device, &color_buffer, config.format);

        Self {
            render_pipeline,
            render_bind_group,

            compute_pipeline,
            compute_bind_group,

            cam_buffer,
            proj_buffer,
            world_buffer,
            color_buffer,
        }
    }
}

pub fn create_compute_pipeline(
    device: &Device,
    color_buffer: &ColorBuffer,
    cam_buffer: &CamBuffer,
    proj_buffer: &ProjBuffer,
    world_buffer: &WorldBuffer,
) -> (ComputePipeline, BindGroup) {
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("raytracing_pipeline::shader_module"),
        source: ShaderSource::Wgsl(COMPUTE_SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("raytracing_pipeline::bind_group_layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: COLOR_BUFFER_FORMAT,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: uniform_binding_type(),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: uniform_binding_type(),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: storage_binding_type(true),
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("raytracing_pipeline::bind_group"),
        layout: &bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&color_buffer.view),
            },
            BindGroupEntry {
                binding: 1,
                resource: cam_buffer.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: proj_buffer.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: world_buffer.0.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("raytracing_pipeline::layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("raytracing_pipeline::pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader_module,
        entry_point: "update",
    });
    (pipeline, bind_group)
}
pub fn create_render_pipeline(
    device: &Device,
    color_buffer: &ColorBuffer,
    format: TextureFormat,
) -> (RenderPipeline, BindGroup) {
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("screen_pipeline::shader_module"),
        source: ShaderSource::Wgsl(SCREEN_SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("screen_pipeline::bind_group_layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::default(),
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("screen_pipeline::bind_group"),
        layout: &bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Sampler(&color_buffer.sampler),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(&color_buffer.view),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("screen_pipeline::layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("screen_pipeline::pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader_module,
            entry_point: "vertex_main",
            buffers: &[],
        },
        fragment: Some(FragmentState {
            module: &shader_module,
            entry_point: "fragment_main",
            targets: &[Some(ColorTargetState {
                format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    });
    (pipeline, bind_group)
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct CamData {
    pos: [f32; 3],
    _padding0: u32,
    rot: [f32; 3],
    _padding1: u32,
    inv_view_mat: [f32; 16],
}
impl CamData {
    pub fn new(cam: &Cam) -> Self {
        Self {
            pos: cam.pos.into(),
            _padding0: 0,
            rot: cam.rot.into(),
            _padding1: 0,
            inv_view_mat: cam.inv_view_mat().0,
        }
    }
}

pub struct CamBuffer(pub Buffer);
impl CamBuffer {
    pub fn new(device: &Device) -> Self {
        Self(device.create_buffer(&BufferDescriptor {
            label: Some("#cam_buffer"),
            size: std::mem::size_of::<CamData>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        }))
    }

    pub fn update(&self, queue: &Queue, cam: &Cam) {
        let data = CamData::new(cam);
        queue.write_buffer(&self.0, 0, cast_slice(&[data]));
    }
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct ProjData {
    size: [u32; 2],
    _padding0: [u32; 2],
    inv_mat: [f32; 16],
}
impl ProjData {
    pub fn new(size: Vec2<u32>, cam: &Cam) -> Self {
        Self {
            size: size.into(),
            _padding0: [0; 2],
            inv_mat: cam.inv_proj_mat(size).0,
        }
    }
}

pub struct ProjBuffer(pub Buffer);
impl ProjBuffer {
    pub fn new(device: &Device) -> Self {
        Self(device.create_buffer(&BufferDescriptor {
            label: Some("#proj_buffer"),
            size: std::mem::size_of::<ProjData>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        }))
    }

    pub fn update(&self, queue: &Queue, size: Vec2<u32>, cam: &Cam) {
        let data = ProjData::new(size, cam);
        queue.write_buffer(&self.0, 0, cast_slice(&[data]));
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct WorldData {
    pub min_chunk_pos: [u32; 3],
    _padding0: [u32; 1],
    pub chunks: [Chunk; WORLD_CHUNKS_COUNT as usize],
}
impl WorldData {
    pub fn new(world: &World) -> Self {
        Self {
            min_chunk_pos: world.min_chunk_pos.into(),
            _padding0: [0; 1],
            chunks: world.chunks.clone(),
        }
    }
}

pub struct WorldBuffer(pub Buffer);
impl WorldBuffer {
    pub fn new(device: &Device) -> Self {
        Self(device.create_buffer(&BufferDescriptor {
            label: Some("#world_buffer"),
            size: std::mem::size_of::<WorldData>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            mapped_at_creation: false,
        }))
    }

    pub fn update(&self, queue: &Queue, world: &World) {
        let data = WorldData::new(world);
        queue.write_buffer(&self.0, 0, bytemuck::cast_slice(&[data]));
    }
}
