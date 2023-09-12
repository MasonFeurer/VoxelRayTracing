pub mod gpu;
pub mod input;
pub mod math;
pub mod player;
pub mod ui;
pub mod world;

use crate::gpu::{egui::Egui, Gpu, GpuResources, Settings, WorldData};
use crate::input::{InputState, Key};
use crate::math::dda::HitResult;
use crate::player::Player;
use crate::world::{Material, Node, NodeSeq, TreeGen, Voxel, World, WorldGen};
use glam::{IVec3, UVec2, Vec3};
use std::time::SystemTime;
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowBuilder};
use world::DEFAULT_VOXEL_MATERIALS;

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

pub fn main() {
    let max_buffer_sizes: u32 = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u32>().expect("invalid integer argument"))
        .unwrap_or(u32::MAX);

    env_logger::init();

    let mut fps_temp: u32 = 0;
    let mut fps: u32 = 0;
    let mut last_second = SystemTime::now();
    let mut last_frame = SystemTime::now();
    let mut input = InputState::default();
    let mut cursor_hidden = true;

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Voxel Ray Tracing Engine")
        .build(&event_loop)
        .unwrap();
    let mut prev_win_size = win_size(&window);
    hide_cursor(&window, true);

    let gpu = pollster::block_on(Gpu::new(&window, max_buffer_sizes));
    let max_nodes = max_buffer_sizes / std::mem::size_of::<Node>() as u32;

    crate::world::open_simplex::init_gradients();

    let mut egui = Egui::new(&window, &gpu);
    let mut game_state = GameState::new(win_size(&window), gpu, max_nodes);

    event_loop.run(move |event, _, flow| match event {
        e if input.update(&e) => {}
        Event::WindowEvent { event, .. } => match event {
            e if !cursor_hidden && egui.winit.on_event(&egui.ctx, &e).consumed => {}
            WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
            _ => {}
        },
        Event::RedrawRequested(_) => {
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
                game_state.update(&input)
            } else {
                UpdateResult::default()
            };

            if input.key_pressed(Key::T) {
                cursor_hidden = !cursor_hidden;
                hide_cursor(&window, cursor_hidden);
            }
            if input.key_pressed(Key::F) {
                toggle_fullscreen(&window);
            }

            let frame_in = FrameInput {
                fps,
                prev_win_size,
                win_size,
            };
            prev_win_size = win_size;

            let frame_rs = game_state.frame(&window, &update_rs, &frame_in, &input, &mut egui);
            match frame_rs {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => println!("SurfaceError: Lost"),
                Err(wgpu::SurfaceError::OutOfMemory) => *flow = ControlFlow::Exit,
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
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}

pub static INVENTORY: &[Voxel] = &[
    Voxel::STONE,
    Voxel::DIRT,
    Voxel::GRASS,
    Voxel::SAND,
    Voxel::MUD,
    Voxel::CLAY,
    Voxel::WOOD,
    Voxel::BARK,
    Voxel::GREEN_LEAVES,
    Voxel::RED_LEAVES,
    Voxel::ORANGE_LEAVES,
    Voxel::YELLOW_LEAVES,
    Voxel::PINK_LEAVES,
    Voxel::FIRE,
    Voxel::MAGMA,
    Voxel::WATER,
    Voxel::BRIGHT,
    Voxel::MIRROR,
    Voxel::GOLD,
    Voxel::ORANGE_TILE,
    Voxel::POLISHED_BLACK_TILES,
    Voxel::SMOOTH_ROCK,
    Voxel::WOOD_FLOORING,
    Voxel::POLISHED_BLACK_FLOORING,
];

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

pub struct GameState {
    pub gpu: Gpu,
    pub gpu_res: GpuResources,
    pub settings: Settings,

    pub player: Player,
    pub inv_sel: u8,

    pub world: World,
    pub world_depth: u32,
    pub world_dirty: bool,

    pub resize_result_tex: bool,
    pub vertical_samples: u32,
    pub path_tracing: bool,

    pub world_gen: WorldGen,
    pub sun_angle: f32,
    pub frame_count: u32,
    pub voxel_materials: Vec<Material>,
}
impl GameState {
    pub fn new(win_size: UVec2, gpu: Gpu, max_nodes: u32) -> Self {
        let win_aspect = win_size.x as f32 / win_size.y as f32;

        let mut settings = Settings::default();
        settings.max_ray_bounces = 3;
        settings.sun_intensity = 4.0;
        settings.sky_color = [0.81, 0.93, 1.0];

        let world_depth = 8;
        let vertical_samples = 400;

        let mut world = World::new(world_depth, max_nodes);

        let tree_gen = TreeGen {
            height: 9..26,
            bark: Voxel::BARK,
            leaves: Voxel::ALL_LEAVES.to_vec(),
            leaves_decay: 0.1,
            branch_count: 1..4,
            branch_height: 0.5..0.8,
            branch_len: 3.0..8.0,
        };
        let world_gen = WorldGen::new(fastrand::i64(..), tree_gen);
        world_gen.populate(IVec3::ZERO, IVec3::splat(world.size as i32), &mut world);

        let result_tex_size = UVec2::new(
            (vertical_samples as f32 * win_aspect) as u32,
            vertical_samples,
        );

        let player = Player::new(
            Vec3::new(world.size as f32 * 0.5, 100.0, world.size as f32 * 0.5),
            0.2,
        );

        settings.sun_pos = Vec3::new(
            0.0f32.to_radians().sin() * 500.0,
            0.0f32.to_radians().cos() * 500.0,
            world.size as f32 * 0.5,
        )
        .to_array();

        let gpu_res =
            GpuResources::new(&gpu, gpu.surface_config.format, result_tex_size, max_nodes);
        gpu_res.buffers.nodes.write(&gpu, 0, world.nodes());
        gpu_res.buffers.settings.write(&gpu, &settings);
        gpu_res
            .buffers
            .world_data
            .write(&gpu, &WorldData::new(&world));

        let voxel_materials = DEFAULT_VOXEL_MATERIALS.to_vec();
        gpu_res
            .buffers
            .voxel_materials
            .write_slice(&gpu, 0, &voxel_materials);

        Self {
            gpu,
            gpu_res,
            settings,

            player,
            inv_sel: 0,
            world,

            world_gen,
            world_depth,
            world_dirty: false,

            vertical_samples,
            resize_result_tex: false,
            path_tracing: false,
            sun_angle: 0.0,
            frame_count: 0,
            voxel_materials,
        }
    }

    pub fn update(&mut self, input: &InputState) -> UpdateResult {
        let mut output = UpdateResult::default();

        let root = self.world.get_node(0);
        let node = move |x: u32, y: u32, z: u32| root.get_child(x + y * 2 + z * 4);
        let edge_dist = 30;

        let mut world_moved = false;
        if (self.player.pos.x as i32) < self.world.min().x + edge_dist {
            world_moved = true;
            self.world.swap_nodes(node(0, 0, 0), node(1, 0, 0));
            self.world.swap_nodes(node(0, 0, 1), node(1, 0, 1));
            self.world.swap_nodes(node(0, 1, 0), node(1, 1, 0));
            self.world.swap_nodes(node(0, 1, 1), node(1, 1, 1));

            self.world.clear_node(node(0, 0, 0));
            self.world.clear_node(node(0, 0, 1));
            self.world.clear_node(node(0, 1, 0));
            self.world.clear_node(node(0, 1, 1));

            self.world.min.x -= self.world.size as i32 / 2;
            self.world_gen.populate(
                self.world.min(),
                self.world.max() - IVec3::X * self.world.size as i32 / 2,
                &mut self.world,
            );
        }
        if (self.player.pos.x as i32) > self.world.max().x - edge_dist {
            world_moved = true;
            self.world.swap_nodes(node(1, 0, 0), node(0, 0, 0));
            self.world.swap_nodes(node(1, 0, 1), node(0, 0, 1));
            self.world.swap_nodes(node(1, 1, 0), node(0, 1, 0));
            self.world.swap_nodes(node(1, 1, 1), node(0, 1, 1));

            self.world.clear_node(node(1, 0, 0));
            self.world.clear_node(node(1, 0, 1));
            self.world.clear_node(node(1, 1, 0));
            self.world.clear_node(node(1, 1, 1));

            self.world.min.x += self.world.size as i32 / 2;
            self.world_gen.populate(
                self.world.min() + IVec3::X * self.world.size as i32 / 2,
                self.world.max(),
                &mut self.world,
            );
        }
        if (self.player.pos.z as i32) < self.world.min().z + edge_dist {
            world_moved = true;
            self.world.swap_nodes(node(0, 0, 0), node(0, 0, 1));
            self.world.swap_nodes(node(0, 1, 0), node(0, 1, 1));
            self.world.swap_nodes(node(1, 0, 0), node(1, 0, 1));
            self.world.swap_nodes(node(1, 1, 0), node(1, 1, 1));

            self.world.clear_node(node(0, 0, 0));
            self.world.clear_node(node(0, 1, 0));
            self.world.clear_node(node(1, 0, 0));
            self.world.clear_node(node(1, 1, 0));

            self.world.min.z -= self.world.size as i32 / 2;
            self.world_gen.populate(
                self.world.min(),
                self.world.max() - IVec3::Z * self.world.size as i32 / 2,
                &mut self.world,
            );
        }
        if (self.player.pos.z as i32) > self.world.max().z - edge_dist {
            world_moved = true;
            self.world.swap_nodes(node(0, 0, 1), node(0, 0, 0));
            self.world.swap_nodes(node(0, 1, 1), node(0, 1, 0));
            self.world.swap_nodes(node(1, 0, 1), node(1, 0, 0));
            self.world.swap_nodes(node(1, 1, 1), node(1, 1, 0));

            self.world.clear_node(node(0, 0, 1));
            self.world.clear_node(node(0, 1, 1));
            self.world.clear_node(node(1, 0, 1));
            self.world.clear_node(node(1, 1, 1));

            self.world.min.z += self.world.size as i32 / 2;
            self.world_gen.populate(
                self.world.min() + IVec3::Z * self.world.size as i32 / 2,
                self.world.max(),
                &mut self.world,
            );
        }
        if world_moved {
            self.gpu_res
                .buffers
                .nodes
                .write(&self.gpu, 0, self.world.nodes());
            self.gpu_res
                .buffers
                .world_data
                .write(&self.gpu, &WorldData::new(&self.world));
        }

        let result_tex_size = self.gpu_res.result_texture.size().as_vec2();

        let prev_pos = self.player.pos;
        let prev_rot = self.player.rot;
        self.player.update(1.0, input, &self.world);
        self.inv_sel = (self.inv_sel as i8 + input.scroll_delta.y as i8)
            .clamp(0, INVENTORY.len() as i8 - 1) as u8;

        if prev_pos != self.player.pos || prev_rot != self.player.rot {
            output.player_moved = true;
        }

        let cam_data = self.player.create_cam_data(result_tex_size);
        self.gpu_res.buffers.cam_data.write(&self.gpu, &cam_data);

        let hit_result = self.player.cast_ray(&self.world);

        let set_vox = if input.left_button_pressed() {
            Some(Voxel::AIR)
        } else if input.right_button_pressed() {
            Some(INVENTORY[self.inv_sel as usize])
        } else {
            None
        };

        let set_pos = match hit_result {
            Some(hit) if input.left_button_pressed() => Some(hit.pos),
            Some(hit) if input.right_button_pressed() => Some(hit.pos + hit.face),
            _ => None,
        };

        if let (Some(pos), Some(vox)) = (set_pos, set_vox) {
            for NodeSeq { idx, count } in self.world.set_voxel(pos, vox).unwrap() {
                self.gpu_res.buffers.nodes.write(
                    &self.gpu,
                    idx as u64,
                    &self.world.nodes()[idx as usize..idx as usize + count as usize],
                );
            }

            self.gpu_res
                .resize_result_texture(&self.gpu, self.gpu_res.result_texture.size());
            self.frame_count = 0;
        }
        output.hit_result = hit_result;
        output
    }

    fn on_resize(&mut self, new_size: UVec2) {
        let prev_result_size = self.gpu_res.result_texture.size();
        let new_aspect = new_size.x as f32 / new_size.y as f32;
        let prev_aspect = prev_result_size.x as f32 / prev_result_size.y as f32;

        self.gpu.resize(new_size);

        if prev_aspect != new_aspect {
            let result_size = UVec2::new(
                (self.vertical_samples as f32 * new_aspect) as u32,
                self.vertical_samples,
            );

            self.gpu_res.resize_result_texture(&self.gpu, result_size);
        }
    }

    pub fn frame(
        &mut self,
        window: &Window,
        update: &UpdateResult,
        frame: &FrameInput,
        input: &InputState,
        egui: &mut Egui,
    ) -> Result<(), wgpu::SurfaceError> {
        if frame.win_size != frame.prev_win_size {
            self.on_resize(frame.win_size);
        }

        if input.key_pressed(Key::Right) && (self.inv_sel as usize) < INVENTORY.len() - 1 {
            self.inv_sel += 1;
        }
        if input.key_pressed(Key::Left) && self.inv_sel > 0 {
            self.inv_sel -= 1;
        }

        let (output, view) = self.gpu.get_output()?;
        let surface_size = self.gpu.surface_size();
        let mut encoder = self.gpu.create_command_encoder();
        let result_tex_size = self.gpu_res.result_texture.size();

        {
            if update.world_changed || update.player_moved {
                self.frame_count = 0;
                self.gpu_res
                    .resize_result_texture(&self.gpu, result_tex_size);
            }

            let buffers = &self.gpu_res.buffers;

            if update.world_changed {
                // buffers.world.write(&self.gpu, &self.world);
            }

            self.frame_count += 1;
            buffers.frame_count.write(&self.gpu, &self.frame_count);

            let cam_data = self.player.create_cam_data(result_tex_size.as_vec2());
            buffers.cam_data.write(&self.gpu, &cam_data);
        }

        let workgroups = result_tex_size / 8;
        match self.path_tracing {
            false => self
                .gpu_res
                .ray_tracer
                .encode_pass(&mut encoder, workgroups),
            true => self
                .gpu_res
                .path_tracer
                .encode_pass(&mut encoder, workgroups),
        }

        self.gpu_res.screen_shader.encode_pass(&mut encoder, &view);

        // --- egui ---
        let egui_textures_free = {
            // --- create scene ---
            let egui_input = egui.winit.take_egui_input(window);

            let egui_output = egui.ctx.run(egui_input, |ctx| {
                let rs = crate::ui::draw_ui(self, frame, update, ctx);
                if rs.clear_result {
                    let aspect = result_tex_size.x as f32 / result_tex_size.y as f32;
                    self.frame_count = 0;
                    let result_size = UVec2::new(
                        (self.vertical_samples as f32 * aspect) as u32,
                        self.vertical_samples,
                    );
                    self.gpu_res.resize_result_texture(&self.gpu, result_size);
                }
            });
            let egui_prims = egui.ctx.tessellate(egui_output.shapes);
            let screen_desc = egui_wgpu::renderer::ScreenDescriptor {
                size_in_pixels: surface_size.into(),
                pixels_per_point: egui_winit::native_pixels_per_point(window),
            };

            // --- update buffers ---
            for (id, image) in egui_output.textures_delta.set {
                egui.wgpu
                    .update_texture(&self.gpu.device, &self.gpu.queue, id, &image);
            }
            egui.wgpu.update_buffers(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &egui_prims,
                &screen_desc,
            );

            // --- render pass ---
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("#egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            egui.wgpu.render(&mut pass, &egui_prims, &screen_desc);

            egui_output.textures_delta.free
        };

        // --- submit passes ---
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // TODO: try freeing textures in the egui scope ^^^^^

        // --- free egui textures ---
        for id in egui_textures_free {
            egui.wgpu.free_texture(&id);
        }

        Ok(())
    }
}
