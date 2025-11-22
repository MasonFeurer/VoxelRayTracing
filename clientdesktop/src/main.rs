pub mod gpu;
pub mod input;
pub mod world;

use crate::gpu::{egui::Egui, CamData, Gpu, GpuResources, Material, Settings, WorldData};
use crate::input::{InputState, Key};

use anyhow::Context;
use client::common::math::HitResult;
use client::common::net::{ClientCmd, ServerCmd};
use client::common::resources::VoxelPack;
use client::common::world::{chunk_to_world_pos, Node};
use client::player::PlayerInput;
use client::world::ClientWorld;
use client::GameState;
use glam::{ivec3, uvec2, vec3, UVec2, Vec3};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use winit::application::ApplicationHandler;
use winit::event::*;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowAttributes, WindowId};

pub fn main() {
    env_logger::init();

    let usage = "blockworld (resource_folder) (username) (port)";
    let mut args = std::env::args();
    _ = args.next(); // First arg is always the path to this program.

    let res_folder = args.next().expect(&format!(
        "Missing cmdline arg \"resource_folder\"\nUsage: {usage}"
    ));

    let username = args
        .next()
        .expect(&format!("Missing cmdline arg \"username\"\nUsage: {usage}"));

    let port = args
        .next()
        .expect(&format!("Missing cmdline arg \"port\"\nUsage: {usage}"));
    let port: u16 = port
        .parse()
        .with_context(|| "Invalid port address")
        .expect(&format!("Invalid cmdline arg \"port\"\nUsage: {usage}"));

    let mut app_state = AppState::new(username, res_folder, port);

    println!("AWSD - Move player");
    println!("F1 - Toggle overlay");
    println!("Z - Toggle flying");

    if let Err(err) = EventLoop::new().unwrap().run_app(&mut app_state) {
        eprintln!("Failed to run app: {err:?}");
    }
}

#[derive(Default)]
pub struct UpdateResult {
    pub hit_result: Option<HitResult>,
    pub world_changed: bool,
    pub player_moved: bool,
}

pub fn hide_cursor(window: &Window, hide: bool) {
    window.set_cursor_visible(!hide);

    let grab_mode = match (hide, cfg!(target_os = "macos")) {
        (false, _) => CursorGrabMode::None,
        (_, true) => CursorGrabMode::Locked,
        (_, false) => CursorGrabMode::Confined,
    };
    _ = window.set_cursor_grab(grab_mode);
}

pub fn load_voxelpack(res_folder: &str) -> anyhow::Result<VoxelPack> {
    let src = std::fs::read_to_string(format!("{res_folder}/voxelpack.ron"))?;
    Ok(client::common::resources::loader::parse_voxelpack(&src)?)
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

pub struct AppState {
    pub window: Option<Arc<Window>>,
    pub gpu: Option<Gpu>,
    pub gpu_res: Option<GpuResources>,
    pub egui: Option<Egui>,
    pub input: InputState,
    timers: Timers,
    pub prev_win_size: UVec2,
    pub cursor_hidden: bool,

    pub voxelpack: VoxelPack,
    pub voxel_mats: Vec<Material>,

    pub settings: Settings,
    pub vertical_samples: u32,

    pub game: GameState,
    hide_overlay: bool,

    pub chunk_requests_sent: std::collections::HashSet<u32>,
}
impl AppState {
    pub fn new(username: String, res_dir: String, port: u16) -> Self {
        let voxelpack = load_voxelpack(&res_dir).unwrap();
        let voxel_mats = load_voxel_mats(&res_dir, &voxelpack).unwrap();

        let mut settings = Settings::default();
        settings.max_ray_bounces = 3;
        settings.sun_intensity = 4.0;
        settings.sky_color = [0.81, 0.93, 1.0];
        settings.samples_per_pixel = 1;

        let world = ClientWorld::new(ivec3(0, 0, 0), 100_000_000, 20);
        let player_pos = world.min_voxel().as_vec3() + Vec3::splat(world.size() as f32) * 0.5;
        let mut game = GameState::new(username, world, player_pos);

        if let Err(err) = game.join_server(SocketAddr::new("127.0.0.1".parse().unwrap(), port)) {
            panic!("Failed to connect to server: {err:?}");
        }

        Self {
            window: None,
            gpu: None,
            gpu_res: None,
            egui: None,
            input: InputState::default(),
            timers: Timers::new(),
            prev_win_size: UVec2::ZERO,
            cursor_hidden: false,

            voxelpack,
            voxel_mats,

            settings,
            vertical_samples: 800,

            game,
            hide_overlay: false,

            chunk_requests_sent: Default::default(),
        }
    }

    fn on_resize(&mut self, new_size: UVec2) {
        let gpu = self.gpu.as_mut().unwrap();
        let gpu_res = self.gpu_res.as_mut().unwrap();

        let prev_result_size = gpu_res.result_texture.size();
        let new_aspect = new_size.x as f32 / new_size.y as f32;
        let prev_aspect = prev_result_size.x as f32 / prev_result_size.y as f32;

        gpu.resize(new_size);

        if prev_aspect != new_aspect {
            let result_size = uvec2(
                (self.vertical_samples as f32 * new_aspect) as u32,
                self.vertical_samples,
            );

            gpu_res.resize_result_texture(gpu, result_size);
        }
    }

    pub fn update_world(&mut self) {
        self.game
            .world
            .center_chunks(self.game.player.pos.as_ivec3());

        let mut chunks = self.game.world.empty_chunks().collect::<Vec<_>>();
        chunks.sort_by(|a, b| {
            let center = self.game.player.pos;
            let a_pos = a.local_pos().as_ivec3() + self.game.world.min_chunk();
            let b_pos = b.local_pos().as_ivec3() + self.game.world.min_chunk();
            let a_dist = center.distance(chunk_to_world_pos(a_pos).as_vec3());
            let b_dist = center.distance(chunk_to_world_pos(b_pos).as_vec3());
            a_dist
                .partial_cmp(&b_dist)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for chunk in chunks {
            if self.chunk_requests_sent.contains(&(chunk.idx() as u32)) {
                continue;
            }

            let global_chunk_pos = chunk.local_pos().as_ivec3() + self.game.world.min_chunk();

            if let Err(err) = self.game.send_cmd(ServerCmd::GetChunkData(
                chunk.idx() as u32,
                global_chunk_pos,
            )) {
                println!("Failed to send cmd to server: {err:?}");
                continue;
            }
            self.chunk_requests_sent.insert(chunk.idx() as u32);
        }

        let mut update_roots = false;
        let mut max_cmds: i32 = 200;
        while max_cmds > 0 {
            max_cmds -= 1;
            match self.game.try_recv_cmd() {
                Ok(Some(ClientCmd::GiveChunkData(idx, pos, nodes))) => {
                    self.chunk_requests_sent.remove(&idx);

                    match self.game.world.create_chunk(pos, &nodes) {
                        Ok(addr) => {
                            self.gpu_res.as_ref().unwrap().buffers.nodes.write(
                                &self.gpu.as_ref().unwrap(),
                                addr as u64,
                                &nodes,
                            );
                            update_roots = true;
                        }
                        Err(_err) => {}
                    };
                }
                Ok(Some(other)) => {
                    println!("Received other cmd from server: {other:?}")
                }
                Ok(None) => (),
                Err(err) => {
                    eprintln!("Error getting commands from server: {err:?}");
                    break;
                }
            }
        }
        if update_roots {
            self.gpu_res.as_ref().unwrap().buffers.chunk_roots.write(
                self.gpu.as_ref().unwrap(),
                0,
                &self.game.world.chunk_roots(),
            );
        }
    }

    pub fn update(input: &InputState, game: &mut GameState) -> UpdateResult {
        let mut output = UpdateResult::default();

        // -------- Player Updates --------
        // Update player pos with input
        {
            let in_ = PlayerInput {
                cursor_movement: input.cursor_delta,
                left: input.key_down(&Key::KeyA),
                right: input.key_down(&Key::KeyD),
                forward: input.key_down(&Key::KeyW),
                backward: input.key_down(&Key::KeyS),
                jump: input.key_down(&Key::Space),
                crouch: input.key_down(&Key::ShiftLeft),
                toggle_fly: input.key_pressed(&Key::KeyZ),
            };
            let player_updates = game.player.process_input(1.0, &in_);
            game.player
                .update(&player_updates, |bb| game.world.get_collisions_w(bb));

            if player_updates.cam_moved || player_updates.frame_vel != Vec3::ZERO {
                output.player_moved = true;
            }
        }
        output
    }

    pub fn frame(&mut self, _update: &UpdateResult) -> Result<(), wgpu::SurfaceError> {
        let (gpu, gpu_res) = (self.gpu.as_ref().unwrap(), self.gpu_res.as_ref().unwrap());
        let egui = self.egui.as_mut().unwrap();
        let window = Arc::clone(self.window.as_ref().unwrap());

        let (output, view) = gpu.get_output()?;
        let mut encoder = gpu.create_command_encoder();
        let result_tex_size = gpu_res.result_texture.size();

        // Upload camera data to GPU
        let buffers = &gpu_res.buffers;

        let cam_data = CamData::create(
            self.game.player.rot,
            self.game.player.eye_pos(),
            self.game.player.fov,
            result_tex_size.as_vec2(),
        );
        buffers.cam_data.write(&gpu, &cam_data);
        buffers
            .chunk_roots
            .write(gpu, 0, &self.game.world.chunk_roots());
        gpu_res
            .buffers
            .world_data
            .write(&gpu, &WorldData::from(&self.game.world));

        let workgroups = result_tex_size / 8;
        gpu_res.ray_tracer.encode_pass(&mut encoder, workgroups);

        gpu_res.screen_shader.encode_pass(&mut encoder, &view);

        // --- egui ---
        if !self.hide_overlay {
            let surface_size = gpu.surface_size();
            // --- create scene ---
            let egui_input = egui.winit.take_egui_input(&window);

            let egui_output = egui.ctx().run(egui_input, |ctx| {
                egui::Area::new(egui::Id::new("area1"))
                    .default_pos(egui::pos2(0.0, 0.0))
                    .movable(true)
                    .show(ctx, |ui| {
                        ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);

                        ui.painter().rect_filled(
                            ui.max_rect(),
                            egui::CornerRadius::same(5),
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200),
                        );
                        ui.add_space(40.0);
                        ui.separator();
                        {
                            ui.heading(format!("FPS: {}", self.timers.fps));
                            ui.heading(format!(
                                "pos: {:.2} {:.2} {:.2}",
                                self.game.player.pos.x,
                                self.game.player.pos.y,
                                self.game.player.pos.z
                            ));
                        }
                        ui.separator();
                        {
                            let (free, capacity) = self.game.world.chunk_alloc_status();
                            let used = ((capacity - free) as f32 / capacity as f32) * 100.0;
                            ui.heading(format!("world size: {}", self.game.world.size_in_chunks()));
                            ui.heading(format!(
                                "chunk count: {} ({})",
                                self.game.world.chunk_count(),
                                self.game.world.populated_count(),
                            ));
                            ui.heading(&format!("memory: %{used:.0}"));
                        }
                    });
            });
            let screen_desc = egui_wgpu::ScreenDescriptor {
                size_in_pixels: surface_size.into(),
                pixels_per_point: egui_winit::pixels_per_point(egui.ctx(), &window),
            };
            let egui_prims = egui
                .ctx()
                .tessellate(egui_output.shapes, screen_desc.pixels_per_point);

            // --- update buffers ---
            for (id, image) in egui_output.textures_delta.set {
                egui.wgpu
                    .update_texture(&gpu.device, &gpu.queue, id, &image);
            }
            egui.wgpu.update_buffers(
                &gpu.device,
                &gpu.queue,
                &mut encoder,
                &egui_prims,
                &screen_desc,
            );

            // --- render pass ---
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("#egui_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    depth_stencil_attachment: None,
                })
                .forget_lifetime();
            egui.wgpu.render(&mut pass, &egui_prims, &screen_desc);

            for id in egui_output.textures_delta.free {
                egui.wgpu.free_texture(&id);
            }
        };

        // --- submit passes ---
        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl ApplicationHandler for AppState {
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window.as_ref().map(|win| win.request_redraw());
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("BlockWorld"))
            .unwrap();
        let window = Arc::new(window);
        let gpu = pollster::block_on(Gpu::new(Arc::clone(&window)));
        let win_size: UVec2 = <[u32; 2]>::from(window.inner_size()).into();

        let max_nodes = gpu.device.limits().max_storage_buffer_binding_size
            / std::mem::size_of::<Node>() as u32;

        let win_aspect = win_size.x as f32 / win_size.y as f32;
        let result_tex_size = uvec2(
            (self.vertical_samples as f32 * win_aspect) as u32,
            self.vertical_samples,
        );

        let gpu_res = GpuResources::new(
            &gpu,
            gpu.surface_config.format,
            result_tex_size,
            max_nodes,
            self.game.world.size_in_chunks(),
        );
        gpu_res
            .buffers
            .nodes
            .write(&gpu, 0, self.game.world.nodes());
        gpu_res
            .buffers
            .chunk_roots
            .write(&gpu, 0, &self.game.world.chunk_roots());
        gpu_res.buffers.settings.write(&gpu, &self.settings);
        gpu_res
            .buffers
            .world_data
            .write(&gpu, &WorldData::from(&self.game.world));
        gpu_res
            .buffers
            .voxel_materials
            .write_slice(&gpu, 0, &self.voxel_mats);

        hide_cursor(&window, true);
        self.cursor_hidden = true;

        self.prev_win_size = win_size;

        self.egui = Some(Egui::new(&window, &gpu));
        self.window = Some(window);
        self.gpu = Some(gpu);
        self.gpu_res = Some(gpu_res);
    }
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.input.on_device_event(&event);
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = Arc::clone(self.window.as_ref().unwrap());
        let egui = self.egui.as_mut().unwrap();
        self.input.on_window_event(&event);

        let mut resize = None;

        match event {
            e if !self.cursor_hidden && egui.winit.on_window_event(&window, &e).consumed => {}
            WindowEvent::CloseRequested => {
                event_loop.exit();
                _ = self.game.disconnect();
            }
            WindowEvent::RedrawRequested => {
                let last_frame_age = SystemTime::now()
                    .duration_since(self.timers.last_frame)
                    .unwrap()
                    .as_millis();

                if last_frame_age < (1000 / 60) {
                    return;
                }
                self.timers.last_frame = SystemTime::now();

                let win_size = <[u32; 2]>::from(window.inner_size()).into();
                if win_size != self.prev_win_size {
                    resize = Some(win_size);
                }
                self.prev_win_size = win_size;

                if self.input.key_pressed(&Key::F1) {
                    self.hide_overlay = !self.hide_overlay;
                }

                self.update_world();
                let update_rs = if self.cursor_hidden {
                    Self::update(&self.input, &mut self.game)
                } else {
                    UpdateResult::default()
                };

                if self.input.key_pressed(&Key::KeyT) {
                    self.cursor_hidden = !self.cursor_hidden;
                    hide_cursor(&window, self.cursor_hidden);
                }
                if self.input.key_pressed(&Key::KeyF) {
                    window.set_fullscreen(match window.fullscreen() {
                        Some(_) => None,
                        None => Some(Fullscreen::Borderless(None)),
                    });
                }

                let frame_rs = self.frame(&update_rs);
                match frame_rs {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => println!("SurfaceError: Lost"),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("{e:?}"),
                };

                self.input.finish_frame();

                self.timers.frame_counter += 1;
                let now = SystemTime::now();
                if now
                    .duration_since(self.timers.last_second)
                    .unwrap()
                    .as_secs()
                    >= 1
                {
                    self.timers.last_second = now;
                    self.timers.fps = self.timers.frame_counter;
                    self.timers.frame_counter = 0;
                }
            }
            _ => {}
        }
        if let Some(size) = resize {
            self.on_resize(size);
        }
    }
}

struct Timers {
    frame_counter: u32,
    fps: u32,
    last_second: SystemTime,
    last_frame: SystemTime,
}
impl Timers {
    fn new() -> Self {
        Self {
            frame_counter: 0,
            fps: 0,
            last_second: SystemTime::now(),
            last_frame: SystemTime::now(),
        }
    }
}
