#![feature(new_uninit)]
#![feature(let_chains)]

pub mod gpu;
pub mod input;
pub mod math;
pub mod player;
pub mod ui;
pub mod world;

use crate::gpu::{egui::Egui, Gpu, GpuResources, Settings};
use crate::input::{InputState, Key};
use crate::math::dda::HitResult;
use crate::player::Player;
use crate::world::{DefaultWorldGen, Material, Voxel, World};
use glam::{UVec2, Vec3};
use winit::window::Window;
use world::DEFAULT_VOXEL_MATERIALS;

pub static INVENTORY: &[Voxel] = &[
    Voxel::STONE,
    Voxel::DIRT,
    Voxel::GRASS,
    Voxel::FIRE,
    Voxel::MAGMA,
    Voxel::WATER,
    Voxel::WOOD,
    Voxel::BARK,
    Voxel::GREEN_LEAVES,
    Voxel::SAND,
    Voxel::MUD,
    Voxel::CLAY,
    Voxel::GOLD,
    Voxel::MIRROR,
    Voxel::BRIGHT,
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

    pub world: Box<World>,
    pub world_depth: u32,
    pub world_dirty: bool,

    pub resize_result_tex: bool,
    pub vertical_samples: u32,
    pub path_tracing: bool,

    pub world_gen: DefaultWorldGen,
    pub sun_angle: f32,
    pub frame_count: u32,
    pub voxel_materials: Vec<Material>,
}
impl GameState {
    pub fn new(win_size: UVec2, gpu: Gpu) -> Self {
        let win_aspect = win_size.x as f32 / win_size.y as f32;

        let mut settings = Settings::default();
        settings.samples_per_pixel = 1;
        settings.max_ray_bounces = 3;
        settings.sun_intensity = 4.0;
        settings.sky_color = [0.81, 0.93, 1.0];

        let world_depth = 7;
        let vertical_samples = 600;

        let mut world = World::new_boxed(world_depth);

        let world_gen = DefaultWorldGen::new(fastrand::i64(..), 1.0, 1.0, 0.001, [9, 20], 6.0);
        _ = world.populate_with(&world_gen);

        let result_tex_size = UVec2::new(
            (vertical_samples as f32 * win_aspect) as u32,
            vertical_samples,
        );

        let player = Player::new(Vec3::new(10.0, 80.0, 10.0), 0.2);

        settings.sun_pos = Vec3::new(
            0.0f32.to_radians().sin() * 500.0,
            0.0f32.to_radians().cos() * 500.0,
            world.size as f32 * 0.5,
        )
        .to_array();

        let gpu_res = GpuResources::new(&gpu, gpu.surface_config.format, result_tex_size);
        gpu_res.buffers.world.write(&gpu, &world);
        gpu_res.buffers.settings.write(&gpu, &settings);

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
            inv_sel: 19,
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

        let result_tex_size = self.gpu_res.result_texture.size().as_vec2();

        let prev_pos = self.player.pos;
        let prev_rot = self.player.rot;
        self.player.update(1.0, input, &self.world);

        if prev_pos != self.player.pos || prev_rot != self.player.rot {
            output.player_moved = true;
        }

        let cam_data = self.player.create_cam_data(result_tex_size);
        self.gpu_res.buffers.cam_data.write(&self.gpu, &cam_data);

        if input.key_pressed(Key::Right) && self.inv_sel < 19 {
            self.inv_sel += 1;
        }
        if input.key_pressed(Key::Left) && self.inv_sel > 0 {
            self.inv_sel -= 1;
        }

        let hit_result = self.player.cast_ray(&self.world);

        if let Some(hit) = hit_result && input.left_button_pressed() {
            if let Ok(()) = self.world.set_voxel(hit.pos, Voxel::AIR) {
                self.gpu_res.buffers.world.write(&self.gpu, &self.world);
                self.gpu_res.resize_result_texture(&self.gpu, self.gpu_res.result_texture.size());
                self.frame_count = 0;
            } else {
                println!("failed to set voxel");
            }
        }
        if let Some(hit) = hit_result && input.right_button_pressed() {
            let voxel_in_hand = INVENTORY[self.inv_sel as usize];
            if let Ok(()) = self.world.set_voxel(hit.pos + hit.face, voxel_in_hand) {
                self.gpu_res.buffers.world.write(&self.gpu, &self.world);
                self.gpu_res.resize_result_texture(&self.gpu, self.gpu_res.result_texture.size());
                self.frame_count = 0;
            } else {
                println!("failed to set voxel");
            }
        }

        output.hit_result = hit_result;
        output
    }

    fn on_resize(&mut self, new_size: UVec2) {
        println!("on_resize");
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
        _input: &InputState,
        egui: &mut Egui,
    ) -> Result<(), wgpu::SurfaceError> {
        if frame.win_size != frame.prev_win_size {
            self.on_resize(frame.win_size);
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
                buffers.world.write(&self.gpu, &self.world);
            }

            self.frame_count += 1;
            buffers.frame_count.write(&self.gpu, &self.frame_count);

            let cam_data = self.player.create_cam_data(result_tex_size.as_vec2());
            buffers.cam_data.write(&self.gpu, &cam_data);

            // for WorldChance { idx, len } in &update.world_changes {
            //     let nodes = self.world.nodes[idx..idx + len];
            //     buffers.world.write_world_nodes(idx, nodes);
            // }
        }

        if self.path_tracing {
            let workgroups = result_tex_size / 8;
            self.gpu_res.raytracer.encode_pass(&mut encoder, workgroups);
        } else {
            let workgroups = result_tex_size / 8;
            self.gpu_res.simple_rt.encode_pass(&mut encoder, workgroups);
        }

        self.gpu_res.result_shader.encode_pass(&mut encoder, &view);

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

        // --- copy result_texture to prev_result_texture ---
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

        // TODO: try freeing textures in the egui scope ^^^^^

        // --- free egui textures ---
        for id in egui_textures_free {
            egui.wgpu.free_texture(&id);
        }

        Ok(())
    }
}
