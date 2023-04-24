use crate::world::World;
use glam::{Mat4, UVec2, Vec2, Vec3};
use wgpu::*;

static RAYTRACER_SRC: &str = include_str!("../../res/raytracer.wgsl");
static OUTPUT_TEX_SHADER_SRC: &str = include_str!("../../res/output_tex_shader.wgsl");
static COLOR_SHADER_SRC: &str = include_str!("../../res/color_shader.wgsl");

const OUTPUT_TEX_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

const COPY_DST: BufferUsages = BufferUsages::COPY_DST;
const COPY_SRC: BufferUsages = BufferUsages::COPY_SRC;
const STORAGE: BufferUsages = BufferUsages::STORAGE;
const UNIFORM: BufferUsages = BufferUsages::UNIFORM;
const VERTEX: ShaderStages = ShaderStages::VERTEX;
const FRAGMENT: ShaderStages = ShaderStages::FRAGMENT;
const COMPUTE: ShaderStages = ShaderStages::COMPUTE;

pub type RandSrc = [f32; 128];

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

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Settings {
    pub max_ray_steps: u32,
    _padding0: [u32; 3],
    pub sky_color: [f32; 4],
    pub sun_pos: [f32; 3],
    pub max_reflections: u32,
    pub shadows: u32,
    _padding1: [u32; 3],
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            max_ray_steps: 100,
            sky_color: [0.3, 0.7, 1.0, 1.0],
            sun_pos: [-1000.0, 1000.0, 0.0],
            max_reflections: 5,
            shadows: 0,
            _padding0: [0; 3],
            _padding1: [0; 3],
        }
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct ColoredVertex {
    pub pos: Vec3,
    pub color: [f32; 4],
}
impl ColoredVertex {
    pub const fn new(pos: Vec3, color: [f32; 4]) -> Self {
        Self { pos, color }
    }

    pub fn attribs() -> [VertexAttribute; 2] {
        vertex_attr_array!(0 => Float32x3, 1 => Float32x4)
    }
}

pub struct OutputTexture(Texture);
impl OutputTexture {
    pub fn new(device: &Device, size: UVec2) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("#output-texture"),
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: OUTPUT_TEX_FORMAT,
            view_formats: &[],
            usage: TextureUsages::COPY_DST // TODO: remove COPY_DST (i dont think its needed)
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING,
        });
        Self(texture)
    }

    pub fn create_view(&self) -> TextureView {
        self.0.create_view(&TextureViewDescriptor::default())
    }

    pub fn create_sampler(&self, device: &Device) -> Sampler {
        device.create_sampler(&SamplerDescriptor {
            label: Some("#output-texture-sampler"),
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
        })
    }

    pub fn size(&self) -> UVec2 {
        let size = self.0.size();
        UVec2::new(size.width, size.height)
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
    ($($binding:expr=>($vis:expr)$entry:expr),*$(,)?) => {{[
        $(BindGroupLayoutEntry {
            binding: $binding,
            visibility: $vis,
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

pub struct ColorShader {
    pub pipeline: RenderPipeline,
    pub bind_group: BindGroup,
    pub model_mat: SimpleBuffer<Mat4>,
    pub view_mat: SimpleBuffer<Mat4>,
    pub proj_mat: SimpleBuffer<Mat4>,
}
impl ColorShader {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let model_mat = SimpleBuffer::new(device, "", COPY_DST | UNIFORM);
        let view_mat = SimpleBuffer::new(device, "", COPY_DST | UNIFORM);
        let proj_mat = SimpleBuffer::new(device, "", COPY_DST | UNIFORM);

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("color-shader.shader-module"),
            source: ShaderSource::Wgsl(COLOR_SHADER_SRC.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("color-shader.bind-group-layout"),
            entries: &bind_group_layout_entries!(
                0 => (VERTEX) uniform_binding_type(),
                1 => (VERTEX) uniform_binding_type(),
                2 => (VERTEX) uniform_binding_type(),
            ),
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("color-shader.bind_group"),
            layout: &bind_group_layout,
            entries: &bind_group_entries!(
                0 => model_mat.0.as_entire_binding(),
                1 => view_mat.0.as_entire_binding(),
                2 => proj_mat.0.as_entire_binding(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("color-shader.pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("color-shader.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<ColoredVertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &ColoredVertex::attribs(),
                }],
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
            model_mat,
            view_mat,
            proj_mat,
        }
    }
}

pub struct OutputTexShader {
    pub pipeline: RenderPipeline,
    pub bind_group: BindGroup,

    pub tex: TextureView,
    pub tex_s: Sampler,
}
impl OutputTexShader {
    pub fn new(
        device: &Device,
        surface_format: TextureFormat,
        tex: TextureView,
        tex_s: Sampler,
    ) -> Self {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("output-tex-shader.shader-module"),
            source: ShaderSource::Wgsl(OUTPUT_TEX_SHADER_SRC.into()),
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
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("output-tex-shader.bind_group"),
            layout: &bind_group_layout,
            entries: &bind_group_entries!(
                0 => BindingResource::TextureView(&tex),
                1 => BindingResource::Sampler(&tex_s),
            ),
        });

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

            tex,
            tex_s,
        }
    }
}

pub struct Raytracer {
    pub pipeline: ComputePipeline,
    pub bind_group: BindGroup,

    pub output_texture: TextureView,
    pub cam_data: SimpleBuffer<CamData>,
    pub settings: SimpleBuffer<Settings>,
    pub world: SimpleBuffer<World>,
    pub rand_src: SimpleBuffer<RandSrc>,
}
impl Raytracer {
    pub fn new(device: &Device, output_texture: TextureView) -> Self {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("#raytracer.shader-module"),
            source: ShaderSource::Wgsl(RAYTRACER_SRC.into()),
        });

        let cam_data = SimpleBuffer::new(device, "", COPY_DST | UNIFORM);
        let settings = SimpleBuffer::new(device, "", COPY_DST | UNIFORM);
        let world = SimpleBuffer::new(device, "", COPY_DST | STORAGE);
        let rand_src = SimpleBuffer::new(device, "", COPY_DST | STORAGE);

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("#raytracer.bind-group-layout"),
            entries: &bind_group_layout_entries!(
                0 => (COMPUTE) BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: OUTPUT_TEX_FORMAT,
                    view_dimension: TextureViewDimension::D2,
                },
                1 => (COMPUTE) uniform_binding_type(),
                2 => (COMPUTE) uniform_binding_type(),
                3 => (COMPUTE) storage_binding_type(true),
                4 => (COMPUTE) storage_binding_type(true),
            ),
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("#raytracer.bind-broup"),
            layout: &bind_group_layout,
            entries: &bind_group_entries!(
                0 => BindingResource::TextureView(&output_texture),
                1 => cam_data.0.as_entire_binding(),
                2 => settings.0.as_entire_binding(),
                3 => world.0.as_entire_binding(),
                4 => rand_src.0.as_entire_binding(),
            ),
        });

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

            output_texture,
            cam_data,
            settings,
            world,
            rand_src,
        }
    }
}

pub struct Shaders {
    pub output_texture: OutputTexture,
    pub color_shader: ColorShader,
    pub output_tex_shader: OutputTexShader,
    pub raytracer: Raytracer,
}
impl Shaders {
    pub fn new(device: &Device, surface_format: TextureFormat, output_size: UVec2) -> Self {
        let output_texture = OutputTexture::new(device, output_size);

        let color_shader = ColorShader::new(device, surface_format);
        let output_tex_shader = OutputTexShader::new(
            device,
            surface_format,
            output_texture.create_view(),
            output_texture.create_sampler(device),
        );
        let raytracer = Raytracer::new(device, output_texture.create_view());

        Self {
            color_shader,
            output_tex_shader,
            raytracer,
            output_texture,
        }
    }
}

pub struct SimpleBuffer<T>(pub Buffer, std::marker::PhantomData<T>);
impl<T> SimpleBuffer<T> {
    pub fn new(device: &Device, label: &str, usage: BufferUsages) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size: std::mem::size_of::<T>() as u64,
            usage,
            mapped_at_creation: false,
        });
        Self(buffer, std::marker::PhantomData)
    }

    pub fn write(&self, queue: &Queue, value: &T) {
        let ptr = value as *const T as *const u8;
        let size = std::mem::size_of::<T>();
        let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
        queue.write_buffer(&self.0, 0, slice);
    }
}
