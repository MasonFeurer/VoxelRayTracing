use crate::gpu::Gpu;
use winit::window::Window;

pub struct Egui {
    pub wgpu: egui_wgpu::renderer::Renderer,
    pub winit: egui_winit::State,
    pub ctx: egui::Context,
}
impl Egui {
    pub fn new(window: &Window, gpu: &Gpu) -> Self {
        let mut winit = egui_winit::State::new(window);
        winit.set_pixels_per_point(egui_winit::native_pixels_per_point(window));

        Self {
            winit,
            wgpu: egui_wgpu::renderer::Renderer::new(
                &gpu.device,
                gpu.surface_config.format,
                None,
                1,
            ),
            ctx: egui::Context::default(),
        }
    }
}
