pub mod debug;
pub mod shaders;

use crate::gpu::shaders::ColoredVertex;
use glam::{UVec2, Vec3};

use wgpu::*;

pub struct Gpu {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub surface_config: SurfaceConfiguration,
}
impl Gpu {
    pub async fn new(window: &winit::window::Window) -> Self {
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
                    features: Features::empty(),
                    limits: Limits {
                        max_storage_buffer_binding_size: 2_000_000_000,
                        max_buffer_size: 2_000_000_000,
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

#[derive(Default)]
pub struct ColoredMesh {
    pub vertices: Vec<ColoredVertex>,
    pub indices: Vec<u32>,
}
impl ColoredMesh {
    pub fn upload(&self, gpu: &Gpu) -> GpuMesh {
        GpuMesh::new(gpu, &self.vertices, &self.indices)
    }

    pub fn quad(&mut self, points: [Vec3; 4], color: [f32; 4]) {
        let idx = self.vertices.len() as u32;
        self.vertices.push(ColoredVertex::new(points[0], color));
        self.vertices.push(ColoredVertex::new(points[1], color));
        self.vertices.push(ColoredVertex::new(points[2], color));
        self.vertices.push(ColoredVertex::new(points[3], color));
        self.indices
            .extend(&[idx, idx + 1, idx + 2, idx + 2, idx + 3, idx]);
    }

    pub fn line(&mut self, from: Vec3, to: Vec3, color: [f32; 4]) {
        let axis = (to - from).abs().normalize();
        let perp = match axis {
            d if d == Vec3::X => [Vec3::Y, Vec3::Z],
            d if d == Vec3::Y => [Vec3::X, Vec3::Z],
            d if d == Vec3::Z => [Vec3::Y, Vec3::X],
            _ => return,
        };
        let offset = 0.02;
        let quad0 = [
            from - perp[0] * offset,
            to - perp[0] * offset,
            to + perp[0] * offset,
            from + perp[0] * offset,
        ];
        let quad1 = [
            from - perp[1] * offset,
            to - perp[1] * offset,
            to + perp[1] * offset,
            from + perp[1] * offset,
        ];
        self.quad(quad0, color);
        self.quad(quad1, color);
    }

    pub fn cube_frame(&mut self, min: Vec3, max: Vec3, color: [f32; 4]) {
        let (a, b) = (min, max);
        self.line(Vec3::new(a.x, a.y, a.z), Vec3::new(b.x, a.y, a.z), color);
        self.line(Vec3::new(b.x, a.y, a.z), Vec3::new(b.x, b.y, a.z), color);
        self.line(Vec3::new(b.x, b.y, a.z), Vec3::new(a.x, b.y, a.z), color);
        self.line(Vec3::new(a.x, b.y, a.z), Vec3::new(a.x, a.y, a.z), color);

        self.line(Vec3::new(a.x, a.y, b.z), Vec3::new(b.x, a.y, b.z), color);
        self.line(Vec3::new(b.x, a.y, b.z), Vec3::new(b.x, b.y, b.z), color);
        self.line(Vec3::new(b.x, b.y, b.z), Vec3::new(a.x, b.y, b.z), color);
        self.line(Vec3::new(a.x, b.y, b.z), Vec3::new(a.x, a.y, b.z), color);

        self.line(Vec3::new(a.x, a.y, a.z), Vec3::new(a.x, a.y, b.z), color);
        self.line(Vec3::new(a.x, b.y, a.z), Vec3::new(a.x, b.y, b.z), color);
        self.line(Vec3::new(b.x, b.y, a.z), Vec3::new(b.x, b.y, b.z), color);
        self.line(Vec3::new(b.x, a.y, a.z), Vec3::new(b.x, a.y, b.z), color);
    }
}
