pub mod graphics;
pub mod input;
pub mod ui;

use ui::UiState;

use crate::input::{InputState, Key};
use graphics::{CamData, Crosshair, Egui, Gpu, GpuResources, Material, Settings, WorldData};

use std::os::unix::fs::PermissionsExt;

use anyhow::Context;
use client::common::resources::{Datapack, Stylepack};
use client::common::world::Node;
use client::net::ServerConn;
use client::player::PlayerInput;
use client::world::ClientWorld;
use client::GameState;
use glam::{ivec3, uvec2, UVec2};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use winit::application::ApplicationHandler;
use winit::event::*;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowAttributes, WindowId};

pub static DEFAULT_META: &str = include_str!("../../stdrespack/meta.ron");
pub static DEFAULT_FEATURES: &str = include_str!("../../stdrespack/features.ron");
pub static DEFAULT_PRESETS: &str = include_str!("../../stdrespack/worldpresets.ron");
pub static DEFAULT_VOXELS: &str = include_str!("../../stdrespack/voxelpack.ron");

pub static DEFAULT_STYLES: &str = include_str!("../../stdrespack/voxelstylepack.ron");

pub static DEFAULT_SERVER_BIN: &'static [u8] =
    include_bytes!("../../stdrespack/blockworld-server-cli");

pub fn local_server_addr() -> SocketAddr {
    SocketAddr::new("127.0.0.1".parse().unwrap(), 60_000)
}

pub fn app_save_dir() -> PathBuf {
    PathBuf::from("/home/mason/.config/blockworld")
}

pub fn setup_config_folder() -> anyhow::Result<()> {
    let root = app_save_dir();
    let datapack_dir = root.join("datapack");
    let stylepack_dir = root.join("stylepack");
    let server_path = root.join("blockworld-server-cli");

    if !root.exists() {
        std::fs::create_dir_all(&root)?;
    }
    if !datapack_dir.exists() {
        std::fs::create_dir(&datapack_dir)?;
        std::fs::write(datapack_dir.join("meta.ron"), DEFAULT_META)?;
        std::fs::write(datapack_dir.join("voxels.ron"), DEFAULT_VOXELS)?;
        std::fs::write(datapack_dir.join("world_features.ron"), DEFAULT_FEATURES)?;
        std::fs::write(datapack_dir.join("world_gen.ron"), DEFAULT_PRESETS)?;
    }
    if !stylepack_dir.exists() {
        std::fs::create_dir(&stylepack_dir)?;
        std::fs::write(stylepack_dir.join("voxel_styles.ron"), DEFAULT_STYLES)?;
        std::fs::write(stylepack_dir.join("meta.ron"), DEFAULT_META)?;
    }
    if !server_path.exists() {
        std::fs::write(&server_path, DEFAULT_SERVER_BIN)?;
        let mut perms = server_path.metadata()?.permissions();
        perms.set_mode(0o700);
        std::fs::set_permissions(server_path, perms)?;
    }
    Ok(())
}

pub fn main() {
    env_logger::init();

    let username = match std::env::home_dir() {
        Some(path) if path.file_name().is_some() => {
            path.file_name().unwrap().to_string_lossy().into_owned()
        }
        _ => String::from("player"),
    };

    if let Err(err) = setup_config_folder() {
        eprintln!("Failed to setup config folder: {err:?}");
        return;
    }

    let datapack = Datapack::load_from(app_save_dir().join("datapack")).unwrap();
    let stylepack = Stylepack::load_from(&datapack, app_save_dir().join("stylepack")).unwrap();

    let server = app_save_dir().join("blockworld-server-cli");
    let mut server_thread = std::process::Command::new(&format!("{}", server.display()))
        .stdout(std::io::stdout())
        .stderr(std::io::stderr())
        .arg(format!("{}", app_save_dir().join("datapack").display()))
        .arg("60000")
        .spawn()
        .unwrap();

    let mut app_state = AppState::new(username, datapack, stylepack);

    println!("AWSD - Move player");
    println!("F1 - Toggle overlay");
    println!("Z - Toggle flying");
    println!("F - Toggle fullscreen");
    println!("T - Toggle cursor");

    if let Err(err) = EventLoop::new().unwrap().run_app(&mut app_state) {
        eprintln!("Failed to run app: {err:?}");
    }
    server_thread.kill();
}

#[derive(Default, Clone)]
pub struct Cursor {
    hidden: bool,
}
impl Cursor {
    pub fn update_win(&self, window: &Window) {
        window.set_cursor_visible(!self.hidden);

        let grab_mode = match (self.hidden, cfg!(target_os = "macos")) {
            (false, _) => CursorGrabMode::None,
            (_, true) => CursorGrabMode::Locked,
            (_, false) => CursorGrabMode::Confined,
        };
        _ = window.set_cursor_grab(grab_mode);
    }

    pub fn hide(&mut self, window: &Window) {
        self.hidden = true;
        self.update_win(window);
    }
    pub fn show(&mut self, window: &Window) {
        self.hidden = false;
        self.update_win(window);
    }

    pub fn toggle(&mut self, window: &Window) {
        self.hidden = !self.hidden;
        self.update_win(window);
    }
}

pub struct AppState {
    pub window: Option<Arc<Window>>,
    pub gpu: Option<Gpu>,
    pub gpu_res: Option<GpuResources>,
    pub egui: Option<Egui>,
    pub input: InputState,
    timers: Timers,
    pub prev_win_size: UVec2,
    pub cursor: Cursor,

    pub datapack: Datapack,
    pub stylepack: Stylepack,

    pub settings: Settings,
    pub vertical_samples: u32,

    pub username: String,
    pub max_nodes: u32,
    pub game: Option<GameState>,
    pub join_game_err: Option<String>,
    pub ui_state: UiState,
    pub crosshair: Crosshair,
    pub freeze_world_anchor: bool,
    hide_overlay: bool,
}
impl AppState {
    pub fn new(username: String, datapack: Datapack, stylepack: Stylepack) -> Self {
        let mut settings = Settings::default();
        settings.max_ray_bounces = 3;
        settings.sun_intensity = 4.0;
        settings.sky_color = [0.81, 0.93, 1.0];

        Self {
            window: None,
            gpu: None,
            gpu_res: None,
            egui: None,
            input: InputState::default(),
            timers: Timers::new(),
            prev_win_size: UVec2::ZERO,
            cursor: Cursor::default(),

            datapack,
            stylepack,

            settings,
            vertical_samples: 800,

            username,
            max_nodes: 0,
            game: None,
            join_game_err: None,
            ui_state: UiState::default(),
            crosshair: Default::default(),
            freeze_world_anchor: false,
            hide_overlay: false,
        }
    }

    pub fn join_game(&mut self, addr: SocketAddr) {
        let world = ClientWorld::new(ivec3(0, 0, 0), self.max_nodes, 40);
        let server = match ServerConn::establish(addr, &self.username) {
            Ok(v) => v,
            Err(e) => {
                self.join_game_err = Some(format!("{e:?}"));
                return;
            }
        };
        let game = GameState::new(self.username.clone(), world, server);

        let gpu = self.gpu.as_ref().unwrap();
        let win_size: UVec2 = <[u32; 2]>::from(self.window.as_ref().unwrap().inner_size()).into();

        let win_aspect = win_size.x as f32 / win_size.y as f32;
        let result_tex_size = uvec2(
            (self.vertical_samples as f32 * win_aspect) as u32,
            self.vertical_samples,
        );
        let gpu_res = GpuResources::new(
            &gpu,
            gpu.surface_config.format,
            result_tex_size,
            self.max_nodes,
            game.world.size_in_chunks(),
        );
        gpu_res.buffers.nodes.write(&gpu, 0, game.world.nodes());
        let mats: Vec<_> = self
            .stylepack
            .voxel_styles
            .styles
            .iter()
            .cloned()
            .map(Material::construct)
            .collect();
        gpu_res.buffers.voxel_materials.write_slice(&gpu, 0, &mats);

        self.gpu_res = Some(gpu_res);
        self.game = Some(game);
        self.cursor.hide(self.window.as_ref().unwrap());
        self.join_game_err = None;
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

    pub fn update_game(&mut self) {
        let Some(game) = &mut self.game else {
            return;
        };
        if !self.freeze_world_anchor {
            game.center_chunks(game.player.pos.as_ivec3());
        }
        game.request_missing_chunks();
    }

    pub fn update(&mut self) {
        // -------- Handle Server Commands -------
        if let Some(game) = &mut self.game {
            let rs = game.process_cmds_timeout(Duration::from_millis(200));
            let rs = match rs {
                Ok(rs) => rs,
                Err(err) => {
                    println!("Encountered error : {err:?}");
                    return;
                }
            };
            for (_pos, root, node_count) in rs.updated_chunks {
                let nodes = &game.world.nodes()[root as usize..root as usize + node_count];
                self.gpu_res.as_ref().unwrap().buffers.nodes.write(
                    &self.gpu.as_ref().unwrap(),
                    root as u64,
                    nodes,
                );
            }
        }
    }

    pub fn update_input(&mut self, delta: f32) {
        let input = &self.input;

        // -------- Player Updates --------
        // Update player pos with input
        if let (Some(game), true) = (&mut self.game, self.cursor.hidden) {
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
            let player_updates = game.player.process_input(delta, &in_);
            game.player.update(&player_updates, |bb| {
                game.world.get_collisions_w(bb, &self.datapack.voxels)
            });
        }

        // --- Misc. key binds ---
        if input.key_pressed(&Key::F1) {
            self.hide_overlay = !self.hide_overlay;
        }
        if input.key_pressed(&Key::F2) {
            self.settings.show_step_count = 1 - self.settings.show_step_count;
        }
        let window = self.window.as_ref().unwrap();
        if input.key_pressed(&Key::KeyT) {
            self.cursor.toggle(&window);
        }
        if input.key_pressed(&Key::KeyF) {
            window.set_fullscreen(match window.fullscreen() {
                Some(_) => None,
                None => Some(Fullscreen::Borderless(None)),
            });
        }
        if input.key_pressed(&Key::KeyQ) {
            self.freeze_world_anchor = !self.freeze_world_anchor;
        }
        if input.key_pressed(&Key::KeyL) {
            self.game = None;
        }
    }

    pub fn draw_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        let gpu = self.gpu.as_ref().unwrap();
        let window = Arc::clone(self.window.as_ref().unwrap());

        let (output, view) = gpu.get_output()?;
        let mut encoder = gpu.create_command_encoder();

        let mut join_game = None;

        // ----- RENDER GAME -----
        if let Some(game) = self.game.as_ref() {
            let gpu_res = self.gpu_res.as_ref().unwrap();
            let result_tex_size = gpu_res.result_texture.size();

            // --- Update buffers ---
            let buffers = &gpu_res.buffers;
            buffers.settings.write(&gpu, &self.settings);
            buffers
                .screen_size
                .write(&gpu, &[result_tex_size.x as f32, result_tex_size.y as f32]);
            buffers.crosshair.write(&gpu, &self.crosshair);
            let cam_data = CamData::create(
                game.player.rot,
                game.player.eye_pos(),
                game.player.fov,
                result_tex_size.as_vec2(),
            );
            buffers.cam_data.write(&gpu, &cam_data);
            buffers.chunk_roots.write(gpu, 0, &game.world.chunk_roots());
            buffers
                .world_data
                .write(&gpu, &WorldData::from(&game.world));

            // --- Render ---
            let workgroups = result_tex_size / 8;
            gpu_res.ray_tracer.encode_pass(&mut encoder, workgroups);
            gpu_res.screen_shader.encode_pass(&mut encoder, &view);
        }
        // ----- RENDER UI -----
        {
            let egui = self.egui.as_mut().unwrap();
            let surface_size = gpu.surface_size();
            let egui_input = egui.winit.take_egui_input(&window);

            let egui_output = egui.ctx().run(egui_input, |ctx| match self.ui_state {
                UiState::GamePlay => {
                    _ = egui::Area::new(egui::Id::new("area1"))
                        .default_pos(egui::pos2(0.0, 0.0))
                        .movable(true)
                        .show(ctx, |ui| {
                            if let Some(game) = &mut self.game {
                                ui::draw_game_overlay(ui, game, &mut self.crosshair, &self.timers);
                            } else {
                                ui.heading("NO GAME STATE!");
                                if let Some(err) = &self.join_game_err {
                                    ui.heading(err);
                                }
                            }
                        })
                }
                UiState::TitleScreen => {
                    _ = egui::CentralPanel::default().show(ctx, |ui| {
                        let rs = ui::draw_title_screen(ui);
                        if let Some(state) = rs.new_ui_state {
                            self.ui_state = state;
                        }
                        if rs.host_game {
                            println!("TOOD: host server");
                        }
                        join_game = rs.join_game;
                    })
                }
                _ => {}
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
        }

        // --- submit render passes ---
        gpu.queue.submit(std::iter::once(encoder.finish()));

        // --- done rendering ---
        output.present();

        if let Some(addr) = join_game {
            self.join_game(addr);
        }
        Ok(())
    }

    pub fn _draw_frame_old(&mut self) -> Result<(), wgpu::SurfaceError> {
        let (gpu, gpu_res) = (self.gpu.as_ref().unwrap(), self.gpu_res.as_ref().unwrap());
        let egui = self.egui.as_mut().unwrap();
        let window = Arc::clone(self.window.as_ref().unwrap());

        let (output, view) = gpu.get_output()?;
        let mut encoder = gpu.create_command_encoder();
        let result_tex_size = gpu_res.result_texture.size();

        // --- Update buffers ---
        let buffers = &gpu_res.buffers;
        buffers.settings.write(&gpu, &self.settings);
        buffers
            .screen_size
            .write(&gpu, &[result_tex_size.x as f32, result_tex_size.y as f32]);
        buffers.crosshair.write(&gpu, &self.crosshair);

        // Upload game state to GPU
        if let Some(game) = &self.game {
            let cam_data = CamData::create(
                game.player.rot,
                game.player.eye_pos(),
                game.player.fov,
                result_tex_size.as_vec2(),
            );
            buffers.cam_data.write(&gpu, &cam_data);
            buffers.chunk_roots.write(gpu, 0, &game.world.chunk_roots());
            buffers
                .world_data
                .write(&gpu, &WorldData::from(&game.world));
        }
        // ----------------------

        let workgroups = result_tex_size / 8;
        gpu_res.ray_tracer.encode_pass(&mut encoder, workgroups);

        gpu_res.screen_shader.encode_pass(&mut encoder, &view);

        // --- egui ---
        let surface_size = gpu.surface_size();
        // --- create scene ---
        let egui_input = egui.winit.take_egui_input(&window);

        let egui_output = egui.ctx().run(egui_input, |ctx| match self.ui_state {
            UiState::GamePlay => {
                _ = egui::Area::new(egui::Id::new("area1"))
                    .default_pos(egui::pos2(0.0, 0.0))
                    .movable(true)
                    .show(ctx, |ui| {
                        if let Some(game) = &mut self.game {
                            ui::draw_game_overlay(ui, game, &mut self.crosshair, &self.timers);
                        } else {
                            ui.heading("NO GAME STATE!");
                        }
                    })
            }
            UiState::TitleScreen => {
                _ = egui::CentralPanel::default().show(ctx, |ui| {
                    let rs = ui::draw_title_screen(ui);
                    if let Some(state) = rs.new_ui_state {
                        self.ui_state = state;
                    }
                })
            }
            _ => {}
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

        self.max_nodes = gpu.device.limits().max_storage_buffer_binding_size
            / std::mem::size_of::<Node>() as u32;

        let win_aspect = win_size.x as f32 / win_size.y as f32;
        let result_tex_size = uvec2(
            (self.vertical_samples as f32 * win_aspect) as u32,
            self.vertical_samples,
        );

        if let Some(game) = &self.game {
            let gpu_res = GpuResources::new(
                &gpu,
                gpu.surface_config.format,
                result_tex_size,
                self.max_nodes,
                game.world.size_in_chunks(),
            );
            gpu_res.buffers.nodes.write(&gpu, 0, game.world.nodes());
            let mats: Vec<_> = self
                .stylepack
                .voxel_styles
                .styles
                .iter()
                .cloned()
                .map(Material::construct)
                .collect();
            gpu_res.buffers.voxel_materials.write_slice(&gpu, 0, &mats);
            self.gpu_res = Some(gpu_res);
            self.cursor.hide(&window);
        }

        self.prev_win_size = win_size;

        self.egui = Some(Egui::new(&window, &gpu));
        self.window = Some(window);
        self.gpu = Some(gpu);
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
            e if !self.cursor.hidden && egui.winit.on_window_event(&window, &e).consumed => {}
            WindowEvent::CloseRequested => {
                event_loop.exit();
                _ = self.game.as_mut().map(GameState::disconnect);
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

                self.update();
                self.update_input(1.0);
                self.update_game();
                match self.draw_frame() {
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

pub struct Timers {
    pub frame_counter: u32,
    pub fps: u32,
    pub last_second: SystemTime,
    pub last_frame: SystemTime,
}
impl Timers {
    pub fn new() -> Self {
        Self {
            frame_counter: 0,
            fps: 0,
            last_second: SystemTime::now(),
            last_frame: SystemTime::now(),
        }
    }
}
