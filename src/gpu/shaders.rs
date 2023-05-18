use crate::world::{VoxelProps, World};
use glam::{Mat4, UVec2, Vec2, Vec3};
use wgpu::*;

static RAYTRACER_SRC: &str = include_str!("../../res/raytracer.wgsl");
static OUTPUT_TEX_SHADER_SRC: &str = include_str!("../../res/output_tex_shader.wgsl");

const OUTPUT_TEX_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

const COPY_DST: BufferUsages = BufferUsages::COPY_DST;
const STORAGE: BufferUsages = BufferUsages::STORAGE;
const UNIFORM: BufferUsages = BufferUsages::UNIFORM;
const FRAGMENT: ShaderStages = ShaderStages::FRAGMENT;
const COMPUTE: ShaderStages = ShaderStages::COMPUTE;

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
    pub samples_per_pixel: u32,
    pub max_ray_steps: u32,
    pub max_ray_bounces: u32,

    pub _padding0: u32,
    pub sky_color: [f32; 3],

    pub _padding1: u32,
    pub sun_pos: [f32; 3],

    pub _padding2: u32,
}

pub struct OutputTexture(pub Texture);
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
            usage: TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
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
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Linear,
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

    pub fn aspect(&self) -> f32 {
        self.size().x as f32 / self.size().y as f32
    }

    pub fn clear(&self, gpu: &crate::gpu::Gpu) {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.clear_texture(
            &self.0,
            &ImageSubresourceRange {
                base_mip_level: 1,
                ..Default::default()
            },
        );
        gpu.queue.submit([encoder.finish()]);
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

pub struct OutputTexShader {
    pub pipeline: RenderPipeline,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,

    pub tex: TextureView,
    pub tex_s: Sampler,
    pub frame_averages: SimpleBuffer<u32>,
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

        let frame_averages =
            SimpleBuffer::<u32>::new(device, "", BufferUsages::UNIFORM | BufferUsages::COPY_DST);

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("output-tex-shader.bind-group-layout"),
            entries: &bind_group_layout_entries!(
                0 => (FRAGMENT) BindingType::Texture {
                    sample_type: TextureSampleType::default(),
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                1 => (FRAGMENT) BindingType::Sampler(SamplerBindingType::Filtering),
                2 => (FRAGMENT) uniform_binding_type(),
            ),
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("output-tex-shader.bind_group"),
            layout: &bind_group_layout,
            entries: &bind_group_entries!(
                0 => BindingResource::TextureView(&tex),
                1 => BindingResource::Sampler(&tex_s),
                2 => frame_averages.0.as_entire_binding(),
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
            bind_group_layout,

            tex,
            tex_s,
            frame_averages,
        }
    }

    pub fn recreate_bind_group(&mut self, device: &Device) {
        self.bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("output-tex-shader.bind_group"),
            layout: &self.bind_group_layout,
            entries: &bind_group_entries!(
                0 => BindingResource::TextureView(&self.tex),
                1 => BindingResource::Sampler(&self.tex_s),
                2 => self.frame_averages.0.as_entire_binding(),
            ),
        });
    }
}

pub struct Raytracer {
    pub pipeline: ComputePipeline,
    pub bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,

    pub output_texture: TextureView,
    pub prev_output_texture: TextureView,

    pub cam_data: SimpleBuffer<CamData>,
    pub settings: SimpleBuffer<Settings>,
    pub world: SimpleBuffer<World>,
    pub voxel_props: SimpleBuffer<[VoxelProps; 256]>,
    pub frame_count: SimpleBuffer<u32>,
}
impl Raytracer {
    pub fn new(
        device: &Device,
        output_texture: TextureView,
        prev_output_texture: TextureView,
    ) -> Self {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("#raytracer.shader-module"),
            source: ShaderSource::Wgsl(RAYTRACER_SRC.into()),
        });

        let cam_data = SimpleBuffer::new(device, "", COPY_DST | UNIFORM);
        let settings = SimpleBuffer::new(device, "", COPY_DST | UNIFORM);
        let world = SimpleBuffer::new(device, "", COPY_DST | STORAGE);
        let voxel_props = SimpleBuffer::new(device, "", COPY_DST | STORAGE);
        let frame_count = SimpleBuffer::new(device, "", COPY_DST | UNIFORM);

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
                5 => (COMPUTE) uniform_binding_type(),
                6 => (COMPUTE) BindingType::Texture {
                    sample_type: TextureSampleType::default(),
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
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
                4 => voxel_props.0.as_entire_binding(),
                5 => frame_count.0.as_entire_binding(),
                6 => BindingResource::TextureView(&prev_output_texture),
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
            bind_group_layout,

            output_texture,
            prev_output_texture,
            cam_data,
            settings,
            world,
            voxel_props,
            frame_count,
        }
    }

    pub fn recreate_bind_group(&mut self, device: &Device) {
        self.bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("#raytracer.bind-broup"),
            layout: &self.bind_group_layout,
            entries: &bind_group_entries!(
                0 => BindingResource::TextureView(&self.output_texture),
                1 => self.cam_data.0.as_entire_binding(),
                2 => self.settings.0.as_entire_binding(),
                3 => self.world.0.as_entire_binding(),
                4 => self.voxel_props.0.as_entire_binding(),
                5 => self.frame_count.0.as_entire_binding(),
                6 => BindingResource::TextureView(&self.prev_output_texture),
            ),
        });
    }
}

pub struct Shaders {
    pub output_texture: OutputTexture,
    pub prev_output_texture: OutputTexture,

    pub output_tex_shader: OutputTexShader,
    pub raytracer: Raytracer,
}
impl Shaders {
    pub fn new(device: &Device, surface_format: TextureFormat, output_size: UVec2) -> Self {
        let output_texture = OutputTexture::new(device, output_size);
        let prev_output_texture = OutputTexture::new(device, output_size);

        let output_tex_shader = OutputTexShader::new(
            device,
            surface_format,
            output_texture.create_view(),
            output_texture.create_sampler(device),
        );
        let raytracer = Raytracer::new(
            device,
            output_texture.create_view(),
            prev_output_texture.create_view(),
        );

        Self {
            output_texture,
            prev_output_texture,

            output_tex_shader,
            raytracer,
        }
    }

    pub fn resize_output_tex(&mut self, device: &Device, new_size: UVec2) {
        self.output_texture = OutputTexture::new(device, new_size);
        self.prev_output_texture = OutputTexture::new(device, new_size);

        self.output_tex_shader.tex = self.output_texture.create_view();
        self.output_tex_shader.tex_s = self.output_texture.create_sampler(device);
        self.output_tex_shader.recreate_bind_group(device);

        self.raytracer.output_texture = self.output_texture.create_view();
        self.raytracer.prev_output_texture = self.prev_output_texture.create_view();
        self.raytracer.recreate_bind_group(device);
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
