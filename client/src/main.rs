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
use crate::world::{
    data::Material,
    gen::{Feature, WorldGen},
    vox_to_chunk_pos, ChunkHeader, Node, Voxel, World, CHUNK_SIZE,
};
use glam::{ivec3, uvec2, uvec3, vec3, IVec3, UVec2, UVec3};
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::{thread::JoinHandle, time::SystemTime};
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowBuilder};

struct RawWorldPtr(usize);
impl RawWorldPtr {
    pub fn new(world: &World) -> Self {
        Self((world as *const World) as usize)
    }

    pub fn get(&self) -> &mut World {
        unsafe { &mut *(self.0 as *mut World) }
    }
}

#[derive(Debug)]
pub struct ChunkBuilder {
    thread: JoinHandle<Option<IVec3>>,
    chunk: ChunkHeader,
    pos: IVec3,
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

    let gpu = pollster::block_on(Gpu::new(&window));
    let limits = gpu.device.limits();
    let max_supported_nodes =
        limits.max_storage_buffer_binding_size / std::mem::size_of::<Node>() as u32;
    let max_nodes = max_supported_nodes.min(u32::MAX);

    crate::world::noise::init_gradients();

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
    Voxel::SNOW,
    Voxel::DEAD_GRASS,
    Voxel::MOIST_GRASS,
    Voxel::SAND,
    Voxel::MUD,
    Voxel::CLAY,
    Voxel::FIRE,
    Voxel::MAGMA,
    Voxel::WATER,
    Voxel::OAK_WOOD,
    Voxel::OAK_LEAVES,
    Voxel::BIRCH_WOOD,
    Voxel::BIRCH_LEAVES,
    Voxel::SPRUCE_WOOD,
    Voxel::SPRUCE_LEAVES,
    Voxel::CACTUS,
    Voxel::GOLD,
    Voxel::MIRROR,
    Voxel::BRIGHT,
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

    pub world_gen: Arc<WorldGen>,
    pub sun_angle: f32,
    pub frame_count: u32,
    pub voxel_materials: Vec<Material>,
    pub dirty_chunks: Vec<IVec3>,
    pub chunk_builders: Vec<ChunkBuilder>,
    pub max_threads: u32,
    pub feature_receiver: Receiver<Feature>,
    pub feature_sender: Sender<Feature>,
    pub features_queue: Vec<Feature>,
    pub build_chunks: bool,
    pub move_world: bool,
}
impl GameState {
    pub fn new(win_size: UVec2, gpu: Gpu, max_nodes: u32) -> Self {
        let win_aspect = win_size.x as f32 / win_size.y as f32;

        let mut settings = Settings::default();
        settings.max_ray_bounces = 3;
        settings.sun_intensity = 4.0;
        settings.sky_color = [0.81, 0.93, 1.0];
        settings.samples_per_pixel = 1;

        let world_depth = 9;
        let world_size = 15;
        let vertical_samples = 800;

        let world = World::new(max_nodes, world_size);

        let world_gen = WorldGen::new(fastrand::i64(..));
        let mut dirty_chunks = vec![];

        for x in 0..world_size {
            for y in 0..world_size {
                for z in 0..world_size {
                    dirty_chunks.push(uvec3(x, y, z).as_ivec3());
                }
            }
        }
        let result_tex_size = uvec2(
            (vertical_samples as f32 * win_aspect) as u32,
            vertical_samples,
        );

        let center = UVec3::splat(world_size * CHUNK_SIZE).as_vec3() * 0.5;
        let player = Player::new(center, 0.2);

        settings.sun_pos = vec3(
            0.0f32.to_radians().sin() * 500.0,
            0.0f32.to_radians().cos() * 500.0,
            world.size() as f32 * 0.5,
        )
        .to_array();

        let gpu_res = GpuResources::new(
            &gpu,
            gpu.surface_config.format,
            result_tex_size,
            max_nodes,
            world_size,
        );
        gpu_res.buffers.nodes.write(&gpu, 0, world.nodes());
        gpu_res.buffers.chunks.write(&gpu, 0, &world.chunks);
        gpu_res.buffers.settings.write(&gpu, &settings);
        gpu_res
            .buffers
            .world_data
            .write(&gpu, &WorldData::from(&world));

        let voxel_materials = world::data::VOXEL_MATERIALS.to_vec();
        gpu_res
            .buffers
            .voxel_materials
            .write_slice(&gpu, 0, &voxel_materials);

        let max_threads = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);

        let (feature_sender, feature_receiver) = channel();

        Self {
            gpu,
            gpu_res,
            settings,

            player,
            inv_sel: 0,
            world,

            world_gen: Arc::new(world_gen),
            world_depth,
            world_dirty: false,

            vertical_samples,
            resize_result_tex: false,
            path_tracing: false,
            sun_angle: 0.0,
            frame_count: 0,
            voxel_materials,
            dirty_chunks,
            chunk_builders: vec![],
            max_threads,
            feature_sender,
            feature_receiver,
            features_queue: vec![],
            build_chunks: true,
            move_world: true,
        }
    }

    pub fn move_world(&mut self) -> bool {
        let regenerate = self.world.update(self.player.pos.as_ivec3());
        let world_moved = !regenerate.is_empty();
        for pos in regenerate {
            self.dirty_chunks.push(pos);

            let idx = self.world.chunk_idx(pos).unwrap();
            let root = self.world.chunks[idx as usize].root;
            *self.world.mut_node(root) = Node::new(Voxel::AIR);
            self.gpu_res.buffers.nodes.write(
                &self.gpu,
                self.world.chunk_nodes_offset(idx) as u64,
                &self.world.chunk_nodes(idx)[0..1],
            );
        }
        if world_moved {
            self.gpu_res
                .buffers
                .world_data
                .write(&self.gpu, &WorldData::from(&self.world));
            self.gpu_res
                .buffers
                .chunks
                .write(&self.gpu, 0, &self.world.chunks);
        }
        world_moved
    }

    pub fn build_dirty_chunks(&mut self) {
        self.dirty_chunks
            .retain(|pos| self.world.chunk_idx(*pos).is_some());
        let player_pos = self.player.pos;
        self.dirty_chunks.sort_by(|a, b| {
            let a_center = (*a * CHUNK_SIZE as i32 + CHUNK_SIZE as i32 / 2).as_vec3();
            let b_center = (*b * CHUNK_SIZE as i32 + CHUNK_SIZE as i32 / 2).as_vec3();
            let a_dist = a_center.distance_squared(player_pos);
            let b_dist = b_center.distance_squared(player_pos);
            b_dist.total_cmp(&a_dist)
        });

        while self.chunk_builders.len() < self.max_threads as usize && self.dirty_chunks.len() > 0 {
            let pos = self.dirty_chunks.pop().unwrap();
            let min = pos * IVec3::splat(CHUNK_SIZE as i32);
            let max = min + IVec3::splat(CHUNK_SIZE as i32);
            let idx = self.world.chunk_idx(pos).unwrap();
            let chunk = self.world.chunks[idx as usize].clone();
            self.world.reset_alloc(chunk.alloc);

            if let Some(voxel) = self.world_gen.chunk_voxel(min, max) {
                // the world gen determined this chunk can be represented by a single voxel type
                *self.world.mut_node(chunk.root) = Node::new(voxel);
                self.gpu_res.buffers.nodes.write(
                    &self.gpu,
                    chunk.root as u64,
                    &self.world.nodes()[chunk.root as usize..chunk.root as usize + 1],
                );
                continue;
            }

            let world_gen = Arc::clone(&self.world_gen);
            let world_ptr = RawWorldPtr::new(&self.world);
            let feature_sender = self.feature_sender.clone();
            let chunk_clone = chunk.clone();

            let thread = std::thread::spawn(move || {
                let world = world_ptr.get();
                // let alloc = chunk_clone.alloc;
                // world.lock_chunk(alloc);

                let rs = match world_gen.build_chunk(chunk_clone, min, world, feature_sender) {
                    Ok(_) => Some(pos),
                    Err(_) => None,
                };

                // let nodes = {
                //     let min = world.chunk_nodes_offset(idx) as usize;
                //     let max = world.allocs[chunk_clone.alloc as usize].range.end;
                //     &mut world.nodes[min as usize..max as usize]
                // };
                // let allocs = &mut world.allocs[chunk_clone.alloc as usize];
                // let rs = world_gen
                //     .build_chunk2(min, allocs, nodes, feature_sender)
                //     .is_ok()
                //     .then_some(pos);

                // world.unlock_chunk(alloc);
                rs
            });
            self.chunk_builders
                .push(ChunkBuilder { pos, chunk, thread });
        }
    }

    pub fn finish_chunk_builders(&mut self, upload_chunks: &mut HashSet<ChunkHeader>) {
        for i in (0..self.chunk_builders.len()).rev() {
            if !self.chunk_builders[i].thread.is_finished() {
                continue;
            }
            let builder = self.chunk_builders.remove(i);
            let Some(_) = builder.thread.join().unwrap() else {
                // The chunk builder failed.
                // Usually because the chunk position left the world.
                continue;
            };
            // Maybe `pos` is not what should be inserted?
            // `pos` was the global chunk coordinate we started writing to.
            // But if the wolrd moved, the region we wrote to may now
            // represent a chunk at a different coordinate.
            upload_chunks.insert(builder.chunk);
        }
    }

    pub fn place_features(&mut self, upload_chunks: &mut HashSet<ChunkHeader>) {
        // if self.chunk_builders.len() > 0 {
        //     return;
        // }

        'f: for i in (0..self.features_queue.len()).rev() {
            let feature = &self.features_queue[i];
            let (min, max) = (feature.min(), feature.max());
            let min_valid = self.world.check_bounds(min).is_ok();
            let max_valid = self.world.check_bounds(max).is_ok();

            if !min_valid && !max_valid {
                // The entire bounds of this feature is out of
                // world bounds, remove it from the queue.
                _ = self.features_queue.remove(i);
                continue;
            }
            if !min_valid || !max_valid {
                // If any of the bounds of this feature is out
                // of bounds, we can't place this feature yet.
                continue;
            }

            // If any chunks occupied by this feature are currently
            // in use by a chunk builder, we can't place this feature because
            // that may cause data races.
            let min_chunk = vox_to_chunk_pos(min) - IVec3::ONE;
            let max_chunk = vox_to_chunk_pos(max) + IVec3::ONE;
            let mut lock_chunks = vec![];

            for x in min_chunk.x..=max_chunk.x {
                for y in min_chunk.y..=max_chunk.y {
                    for z in min_chunk.z..=max_chunk.z {
                        let Some(chunk_idx) = self.world.chunk_idx(ivec3(x, y, z)) else {
                            continue 'f;
                        };
                        let chunk = self.world.chunks[chunk_idx as usize].clone();

                        if upload_chunks.contains(&chunk) {
                            continue 'f;
                        }
                        // if self.chunk_builders.len() > 0 {
                        //     continue 'f;
                        // }

                        // // This shouldn't fail because we just made
                        // // sure `min` and `max` were in bounds.
                        // let chunk_idx = self.world.chunk_idx(ivec3(x, y, z)).unwrap();
                        if self.chunk_builders.iter().any(|b| {
                            b.chunk.alloc == chunk.alloc
                                || b.chunk.root == chunk.root
                                || b.pos == ivec3(x, y, z)
                        }) {
                            continue 'f;
                        }
                        lock_chunks.push(chunk.alloc);
                    }
                }
            }
            // We can now remove it from the queue and place it.

            for idx in &lock_chunks {
                self.world.lock_chunk(*idx);
            }
            let feature = self.features_queue.remove(i);
            feature.place(|pos, vox| {
                let chunk_pos = vox_to_chunk_pos(pos);
                let Some(chunk_idx) = self.world.chunk_idx(chunk_pos) else {
                    return;
                };

                let chunk = &self.world.chunks[chunk_idx as usize];
                assert!(!self
                    .chunk_builders
                    .iter()
                    .any(|b| b.chunk.alloc == chunk.alloc
                        || b.chunk.root == chunk.root
                        || b.pos == chunk_pos));

                upload_chunks.insert(chunk.clone());
                _ = self.world.set_voxel(pos, vox, |_| {});
            });
            for idx in lock_chunks {
                self.world.unlock_chunk(idx);
            }
        }
    }

    fn check_player_interactions(&mut self, input: &InputState) -> Option<HitResult> {
        let hit_result = self.player.cast_ray(&self.world);

        enum Action {
            Place,
            Break,
        }
        let action = if input.left_button_pressed()
            || (input.left_button_down() & input.key_down(Key::LControl))
        {
            Some(Action::Break)
        } else if input.right_button_pressed()
            || (input.right_button_down() & input.key_down(Key::LControl))
        {
            Some(Action::Place)
        } else {
            None
        };

        let set_vox = match action {
            Some(Action::Break) => Some(Voxel::AIR),
            Some(Action::Place) => Some(INVENTORY[self.inv_sel as usize]),
            None => None,
        };
        let set_pos = match (action, hit_result) {
            (Some(Action::Break), Some(hit)) => Some(hit.pos),
            (Some(Action::Place), Some(hit)) => Some(hit.pos + hit.face),
            _ => None,
        };

        if let (Some(pos), Some(vox)) = (set_pos, set_vox) {
            for range in self.world.set_voxel_collected(pos, vox).unwrap() {
                self.gpu_res.buffers.nodes.write(
                    &self.gpu,
                    range.start as u64,
                    &self.world.nodes()[range.start as usize..range.end as usize],
                );
            }

            self.gpu_res
                .resize_result_texture(&self.gpu, self.gpu_res.result_texture.size());
            self.frame_count = 0;
        }
        hit_result
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

        // Receive and store features from builder threads
        while let Ok(feature) = self.feature_receiver.try_recv() {
            self.features_queue.push(feature);
        }

        // -------- World Updates --------
        {
            // A collection of chunks that will need to
            // be uploaded to the GPU.
            let mut upload_chunks = HashSet::<ChunkHeader>::new();

            // Move local world origin to follow player
            let _world_moved = match self.move_world {
                true => self.move_world(),
                false => false,
            };

            self.finish_chunk_builders(&mut upload_chunks);
            if self.build_chunks {
                self.build_dirty_chunks();
            }
            self.place_features(&mut upload_chunks);

            // Upload collected chunks in `upload_chunks`
            for chunk in upload_chunks {
                if self.chunk_builders.iter().any(|b| b.chunk == chunk) {
                    panic!(
                        "can't upload chunk to GPU because it is being used in a builder thread."
                    );
                }
                let node_range =
                    chunk.root as usize..self.world.allocs[chunk.alloc as usize].range.end as usize;
                self.gpu_res.buffers.nodes.write(
                    &self.gpu,
                    chunk.root as u64,
                    &self.world.nodes()[node_range],
                );
            }
        }

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
        if input.key_pressed(Key::N) {
            self.move_world ^= true;
        }
        if input.key_pressed(Key::M) {
            self.build_chunks ^= true;
        }

        // Handle player interactions with input
        output.hit_result = self.check_player_interactions(input);
        output
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

        // Update voxel selection with scroll wheel or Up/Down
        if (input.scroll_delta.y < 0.0 || input.key_pressed(Key::Down)) && self.inv_sel > 0 {
            self.inv_sel -= 1;
        }
        if (input.scroll_delta.y > 0.0 || input.key_pressed(Key::Up))
            && self.inv_sel < (INVENTORY.len() - 1) as u8
        {
            self.inv_sel += 1;
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

            buffers.frame_count.write(&self.gpu, &self.frame_count);
            self.frame_count += 1;

            // Upload camera data to GPU
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
        {
            // --- create scene ---
            let egui_input = egui.winit.take_egui_input(window);

            let egui_output = egui.ctx.run(egui_input, |ctx| {
                let rs = crate::ui::draw_ui(self, frame, update, ctx);
                if rs.clear_result {
                    let aspect = result_tex_size.x as f32 / result_tex_size.y as f32;
                    self.frame_count = 0;
                    let result_size = uvec2(
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
            std::mem::drop(pass);

            for id in egui_output.textures_delta.free {
                egui.wgpu.free_texture(&id);
            }
        };

        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.gpu_res.result_texture.handle,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &self.gpu_res.prev_result_texture.handle,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.gpu_res.result_texture.size().x,
                height: self.gpu_res.result_texture.size().y,
                depth_or_array_layers: 1,
            },
        );

        // --- submit passes ---
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
