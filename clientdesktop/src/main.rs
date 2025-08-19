pub mod gpu;
pub mod input;
pub mod player;
pub mod world;

use crate::gpu::{egui::Egui, Gpu, GpuResources, Material, Settings, WorldData};
use crate::input::{InputState, Key};
use crate::player::Player;
use crate::world::{Node, World};

use client::common::math::HitResult;
use client::common::resources::VoxelPack;
use client::common::Voxel;
use client::GameState;
use glam::{ivec3, uvec2, uvec3, vec3, UVec2};
use std::time::SystemTime;
use winit::event::*;
use winit::event_loop::EventLoop;
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowAttributes};

pub struct FrameInput {
    pub fps: u32,
    pub prev_win_size: UVec2,
    pub win_size: UVec2,
}

#[derive(Default)]
pub struct UpdateResult {
    pub hit_result: Option<HitResult>,
    pub world_changed: bool,
    pub player_moved: bool,
}

pub static LOCAL_SERVER_ADDR: &str = "127.0.0.1:60000";

pub fn hide_cursor(window: &Window, hide: bool) {
    window.set_cursor_visible(!hide);

    let grab_mode = match (hide, cfg!(target_os = "macos")) {
        (false, _) => CursorGrabMode::None,
        (_, true) => CursorGrabMode::Locked,
        (_, false) => CursorGrabMode::Confined,
    };
    _ = window.set_cursor_grab(grab_mode);
}
pub fn toggle_fullscreen(window: &Window) {
    window.set_fullscreen(match window.fullscreen() {
        Some(_) => None,
        None => Some(Fullscreen::Borderless(None)),
    });
}
pub fn win_size(window: &Window) -> UVec2 {
    UVec2::from(<[u32; 2]>::from(window.inner_size()))
}

pub fn load_voxelpack(res_folder: &str) -> anyhow::Result<VoxelPack> {
    let src = std::fs::read_to_string(format!("{res_folder}/voxelpack.ron"))?;
    client::common::resources::loader::parse_voxelpack(&src)
}

pub fn load_voxel_mats(res_folder: &str, voxels: &VoxelPack) -> anyhow::Result<Vec<Material>> {
    let src = std::fs::read_to_string(format!("{res_folder}/voxelstylepack.ron"))?;
    let styles = client::common::resources::loader::parse_voxel_stylepack(&src, voxels)?;

    let mut materials = Vec::with_capacity(styles.styles.len());
    for style in styles.styles {
        materials.push(Material {
            color: style.color,
            empty: style.empty as u32,
            scatter: 0.0,
            emission: 0.0,
            polish_bounce_chance: 0.0,
            translucency: 0.0,
            polish_color: [0.0; 3],
            polish_scatter: 0.0,
        });
    }
    Ok(materials)
}

pub struct AppState<'a> {
    pub gpu: Gpu<'a>,
    pub gpu_res: GpuResources,
    pub settings: Settings,
    pub vertical_samples: u32,

    pub player: Player,
    pub world: World,

    pub game: GameState,
}
impl<'a> AppState<'a> {
    pub fn new(
        res_dir: String,
        game: GameState,
        win_size: UVec2,
        gpu: Gpu<'a>,
        max_nodes: u32,
    ) -> Self {
        let win_aspect = win_size.x as f32 / win_size.y as f32;

        let voxelpack = load_voxelpack(&res_dir).unwrap();
        let voxel_mats = load_voxel_mats(&res_dir, &voxelpack).unwrap();

        let mut settings = Settings::default();
        settings.max_ray_bounces = 3;
        settings.sun_intensity = 4.0;
        settings.sky_color = [0.81, 0.93, 1.0];
        settings.samples_per_pixel = 1;

        let vertical_samples = 400;

        let result_tex_size = uvec2(
            (vertical_samples as f32 * win_aspect) as u32,
            vertical_samples,
        );

        let world_size = 10;
        let mut world = World::new(ivec3(0, 0, 0), 100_000, world_size);
        // Create a world (in-dev)
        world.put_chunk(uvec3(0, 0, 0), &[Node::new(Voxel::EMPTY)]);
        let stone = voxelpack.by_name("stone").unwrap();
        let grass = voxelpack.by_name("grass").unwrap();
        let sand = voxelpack.by_name("sand").unwrap();

        _ = world.set_voxel(ivec3(0, 0, 0), stone);
        _ = world.set_voxel(ivec3(1, 0, 0), stone);
        _ = world.set_voxel(ivec3(0, 0, 1), stone);

        _ = world.set_voxel(ivec3(0, 1, 0), grass);
        _ = world.set_voxel(ivec3(0, 1, 1), sand);
        _ = world.set_voxel(ivec3(1, 1, 0), sand);

        println!(
            "{:?}",
            world.chunks.chunks[0].as_ref().unwrap().alloc.free_mem[0]
        );

        let mut player = Player::new(vec3(1.0, 0.0, 1.0), 0.2);
        player.flying = true;

        let gpu_res = GpuResources::new(
            &gpu,
            gpu.surface_config.format,
            result_tex_size,
            max_nodes,
            world_size,
        );
        gpu_res.buffers.nodes.write(&gpu, 0, world.nodes());
        gpu_res
            .buffers
            .chunk_roots
            .write(&gpu, 0, &world.chunk_roots());
        gpu_res.buffers.settings.write(&gpu, &settings);
        gpu_res
            .buffers
            .world_data
            .write(&gpu, &WorldData::from(&world));
        gpu_res
            .buffers
            .voxel_materials
            .write_slice(&gpu, 0, &voxel_mats);

        Self {
            gpu,
            gpu_res,
            settings,
            vertical_samples,
            game,

            player,
            world,
        }
    }

    fn on_resize(&mut self, new_size: UVec2) {
        let prev_result_size = self.gpu_res.result_texture.size();
        let new_aspect = new_size.x as f32 / new_size.y as f32;
        let prev_aspect = prev_result_size.x as f32 / prev_result_size.y as f32;

        self.gpu.resize(new_size);

        if prev_aspect != new_aspect {
            let result_size = uvec2(
                (self.vertical_samples as f32 * new_aspect) as u32,
                self.vertical_samples,
            );

            self.gpu_res.resize_result_texture(&self.gpu, result_size);
        }
    }

    pub fn update(&mut self, input: &InputState) -> UpdateResult {
        let mut output = UpdateResult::default();

        // -------- Player Updates --------
        // Update player pos with input
        {
            let prev_pos = self.player.pos;
            let prev_rot = self.player.rot;
            self.player.update(1.0, input, &self.world);

            if prev_pos != self.player.pos || prev_rot != self.player.rot {
                output.player_moved = true;
            }
        }

        // Toggle settings with key presses
        // if input.key_pressed(Key::N) {
        // self.move_world ^= true;
        // }
        // if input.key_pressed(Key::M) {
        // self.build_chunks ^= true;
        // }

        // Handle player interactions with input
        // output.hit_result = self.check_player_interactions(input);
        output
    }

    pub fn frame(
        &mut self,
        _window: &Window,
        _update: &UpdateResult,
        frame: &FrameInput,
        _input: &InputState,
        _egui: &mut Egui,
    ) -> Result<(), wgpu::SurfaceError> {
        if frame.win_size != frame.prev_win_size {
            self.on_resize(frame.win_size);
        }

        let (output, view) = self.gpu.get_output()?;
        let mut encoder = self.gpu.create_command_encoder();
        let result_tex_size = self.gpu_res.result_texture.size();

        // Upload camera data to GPU
        let buffers = &self.gpu_res.buffers;
        let cam_data = self.player.create_cam_data(result_tex_size.as_vec2());
        buffers.cam_data.write(&self.gpu, &cam_data);

        let workgroups = result_tex_size / 8;
        self.gpu_res
            .ray_tracer
            .encode_pass(&mut encoder, workgroups);

        self.gpu_res.screen_shader.encode_pass(&mut encoder, &view);

        // // --- egui ---
        // {
        //     let surface_size = self.gpu.surface_size();
        //     // --- create scene ---
        //     let egui_input = egui.winit.take_egui_input(window);

        //     let egui_output = egui.ctx.run(egui_input, |ctx| {
        //         let rs = crate::ui::draw_ui(self, frame, update, ctx);
        //         if rs.clear_result {
        //             let aspect = result_tex_size.x as f32 / result_tex_size.y as f32;
        //             self.frame_count = 0;
        //             let result_size = uvec2(
        //                 (self.vertical_samples as f32 * aspect) as u32,
        //                 self.vertical_samples,
        //             );
        //             self.gpu_res.resize_result_texture(&self.gpu, result_size);
        //         }
        //     });
        //     let egui_prims = egui.ctx.tessellate(egui_output.shapes);
        //     let screen_desc = egui_wgpu::renderer::ScreenDescriptor {
        //         size_in_pixels: surface_size.into(),
        //         pixels_per_point: egui_winit::native_pixels_per_point(window),
        //     };

        //     // --- update buffers ---
        //     for (id, image) in egui_output.textures_delta.set {
        //         egui.wgpu
        //             .update_texture(&self.gpu.device, &self.gpu.queue, id, &image);
        //     }
        //     egui.wgpu.update_buffers(
        //         &self.gpu.device,
        //         &self.gpu.queue,
        //         &mut encoder,
        //         &egui_prims,
        //         &screen_desc,
        //     );

        //     // --- render pass ---
        //     let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //         label: Some("#egui_render_pass"),
        //         color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        //             view: &view,
        //             resolve_target: None,
        //             ops: wgpu::Operations {
        //                 load: wgpu::LoadOp::Load,
        //                 store: true,
        //             },
        //         })],
        //         depth_stencil_attachment: None,
        //     });
        //     egui.wgpu.render(&mut pass, &egui_prims, &screen_desc);
        //     std::mem::drop(pass);

        //     for id in egui_output.textures_delta.free {
        //         egui.wgpu.free_texture(&id);
        //     }
        // };

        // --- submit passes ---
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub fn main() {
    env_logger::init();

    let res_dir = std::env::args()
        .nth(1)
        .expect("Missing cmdline arg for resource directory path");

    let mut fps_temp: u32 = 0;
    let mut fps: u32 = 0;
    let mut last_second = SystemTime::now();
    let mut last_frame = SystemTime::now();
    let mut input = InputState::default();
    let mut cursor_hidden = true;

    let event_loop = EventLoop::new().unwrap();

    let window = event_loop
        .create_window(WindowAttributes::default().with_title("BlockWorld"))
        .unwrap();

    let mut prev_win_size = win_size(&window);
    hide_cursor(&window, true);

    let gpu = pollster::block_on(Gpu::new(&window));
    let limits = gpu.device.limits();
    let max_supported_nodes =
        limits.max_storage_buffer_binding_size / std::mem::size_of::<Node>() as u32;
    let max_nodes = max_supported_nodes.min(u32::MAX);

    let mut egui = Egui::new(&window, &gpu);
    let mut app_state = AppState::new(
        res_dir,
        GameState::new(String::from("GOOD_USERNAME")),
        win_size(&window),
        gpu,
        max_nodes,
    );

    _ = event_loop.run(|event, ael| match event {
        e if input.update(&e) => {}
        Event::WindowEvent { event, .. } => match event {
            e if !cursor_hidden && egui.winit.on_window_event(&window, &e).consumed => {}
            WindowEvent::CloseRequested => ael.exit(),
            WindowEvent::RedrawRequested => {
                let last_frame_age = SystemTime::now()
                    .duration_since(last_frame)
                    .unwrap()
                    .as_millis();

                if last_frame_age < (1000 / 60) {
                    return;
                }
                last_frame = SystemTime::now();
                let win_size = win_size(&window);

                let update_rs = if cursor_hidden {
                    app_state.update(&input)
                } else {
                    UpdateResult::default()
                };

                if input.key_pressed(&Key::Character("t".into())) {
                    cursor_hidden = !cursor_hidden;
                    hide_cursor(&window, cursor_hidden);
                }
                if input.key_pressed(&Key::Character("f".into())) {
                    toggle_fullscreen(&window);
                }

                let frame_in = FrameInput {
                    fps,
                    prev_win_size,
                    win_size,
                };
                prev_win_size = win_size;

                let frame_rs = app_state.frame(&window, &update_rs, &frame_in, &input, &mut egui);
                match frame_rs {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => println!("SurfaceError: Lost"),
                    Err(wgpu::SurfaceError::OutOfMemory) => ael.exit(),
                    Err(e) => eprintln!("{e:?}"),
                };

                input.finish_frame();

                fps_temp += 1;
                let now = SystemTime::now();
                if now.duration_since(last_second).unwrap().as_secs() >= 1 {
                    last_second = now;
                    fps = fps_temp;
                    fps_temp = 0;
                }
            }
            _ => {}
        },
        Event::AboutToWait => {
            window.request_redraw();
        }
        _ => {}
    });
}
