use crate::gpu::Gpu;
use winit::event_loop::EventLoop;

pub struct Egui {
    pub wgpu: egui_wgpu::renderer::Renderer,
    pub winit: egui_winit::State,
    pub ctx: egui::Context,
}
impl Egui {
    pub fn new(event_loop: &EventLoop<()>, gpu: &Gpu) -> Self {
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
