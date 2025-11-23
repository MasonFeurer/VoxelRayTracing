use super::texture::Texture;
use super::{CamData, Crosshair, Gpu, Material, NodeAddr, Settings, WorldData};
use client::common::world::Node;
use glam::UVec2;
use wgpu::*;

pub struct Buffers {
    pub cam_data: SimpleBuffer<CamData>,
    pub settings: SimpleBuffer<Settings>,
    pub world_data: SimpleBuffer<WorldData>,
    pub nodes: ArrayBuffer<Node>,
    pub voxel_materials: SimpleBuffer<[Material; 256]>,
    pub chunk_roots: ArrayBuffer<NodeAddr>,

    pub screen_size: SimpleBuffer<[f32; 2]>,
    pub crosshair: SimpleBuffer<Crosshair>,
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
            chunk_roots: ArrayBuffer::new(gpu, "chunk_roots", COPY_DST | STORAGE, chunk_count),

            screen_size: SimpleBuffer::new(gpu, "screen_size", COPY_DST | UNIFORM),
            crosshair: SimpleBuffer::new(gpu, "crosshair", COPY_DST | UNIFORM),
        }
    }

    pub fn resize_chunk_buffer(&mut self, gpu: &Gpu, world_size: u32) {
        const COPY_DST: BufferUsages = BufferUsages::COPY_DST;
        const STORAGE: BufferUsages = BufferUsages::STORAGE;

        let chunk_count = world_size * world_size * world_size;
        self.chunk_roots = ArrayBuffer::new(gpu, "chunk_roots", COPY_DST | STORAGE, chunk_count);
    }
    pub fn resize_node_buffer(&mut self, gpu: &Gpu, max_nodes: u32) {
        const COPY_DST: BufferUsages = BufferUsages::COPY_DST;
        const STORAGE: BufferUsages = BufferUsages::STORAGE;

        self.nodes = ArrayBuffer::new(gpu, "nodes", COPY_DST | STORAGE, max_nodes);
    }

    pub fn update_data(&mut self) {
        todo!()
    }
}

pub const RESULT_TEX_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
pub const RESULT_TEX_USAGES: TextureUsages = TextureUsages::COPY_DST
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
    pub fn new(
        src: &str,
        gpu: &Gpu,
        tex: &Texture,
        surface_format: TextureFormat,
        buffers: &Buffers,
    ) -> Self {
        let device = &gpu.device;
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("screen-shader.shader-module"),
            source: ShaderSource::Wgsl(src.into()),
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
                2 => (FRAGMENT) uniform_binding_type(),
                3 => (FRAGMENT) uniform_binding_type(),
            ),
        });
        let bind_group = Self::create_bind_group(gpu, &bind_group_layout, tex, buffers);

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

    pub fn create_bind_group(
        gpu: &Gpu,
        layout: &BindGroupLayout,
        tex: &Texture,
        buffers: &Buffers,
    ) -> BindGroup {
        gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("screen-shader.bind_group"),
            layout,
            entries: &bind_group_entries!(
                0 => BindingResource::TextureView(&tex.view),
                1 => BindingResource::Sampler(&tex.sampler),
                2 => buffers.screen_size.0.as_entire_binding(),
                3 => buffers.crosshair.0.as_entire_binding(),
            ),
        })
    }

    pub fn recreate_bind_group(&mut self, gpu: &Gpu, tex: &Texture, buffers: &Buffers) {
        self.bind_group = Self::create_bind_group(gpu, &self.bind_group_layout, tex, buffers);
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
                7 => buffers.chunk_roots.0.as_entire_binding(),
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
