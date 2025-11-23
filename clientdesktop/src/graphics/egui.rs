use super::Gpu;
use winit::window::Window;

pub struct Egui {
    pub wgpu: egui_wgpu::Renderer,
    pub winit: egui_winit::State,
}
impl Egui {
    pub fn new(window: &Window, gpu: &Gpu) -> Self {
        let ctx = egui::Context::default();
        let ppp = egui_winit::pixels_per_point(&ctx, window);

        let winit =
            egui_winit::State::new(ctx, egui::ViewportId::ROOT, window, Some(ppp), None, None);

        Self {
            winit,
            wgpu: egui_wgpu::Renderer::new(&gpu.device, gpu.surface_config.format, None, 1, false),
        }
    }

    pub fn ctx(&self) -> &egui::Context {
        self.winit.egui_ctx()
    }
}
