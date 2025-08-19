use crate::gpu::CamData;
use crate::input::{InputState, Key, NamedKey};
use crate::world::{Voxel, World};
use client::common::math::Aabb;
use client::common::math::{axis_rot_to_ray, cast_ray, HitResult};
use glam::{vec2, vec3, BVec3, Mat4, Vec2, Vec3};

const GRAVITY: f32 = -0.060;

#[derive(Clone)]
pub struct Player {
    pub fov: f32,

    pub flying: bool,
    pub on_ground: bool,

    pub pos: Vec3,
    // (in degrees)
    pub rot: Vec3,
    pub vel: Vec3,
    pub speed: f32,
}
impl Player {
    pub fn new(pos: Vec3, speed: f32) -> Self {
        Self {
            fov: 70.0,

            flying: false,
            on_ground: false,

            pos,
            rot: Vec3::ZERO,
            vel: Vec3::ZERO,
            speed,
        }
    }

    pub fn handle_cursor_movement(&mut self, t_delta: f32, delta: Vec2) {
        const SENSITIVITY: f32 = 0.4;
        let delta = delta * t_delta;

        // in model space, the camera is looking negative along the Z axis, so
        // moving the cursor up/down corresponds to rotation about the X axis
        self.rot.x += SENSITIVITY * delta.y;
        self.rot.x = self.rot.x.clamp(-90.0, 90.0);

        // moving the cursor left/right corresponds to rotation about the Y axis
        self.rot.y -= SENSITIVITY * delta.x;

        // the camera does not rotate about the Z axis. That would be like tilting your head
    }

    pub fn create_aabb(&self) -> Aabb {
        const WIDTH: f32 = 0.6;
        const HEIGHT: f32 = 3.8;

        Aabb::new(
            self.pos - vec3(WIDTH, 0.0, WIDTH) * 0.5,
            self.pos + vec3(WIDTH, HEIGHT * 2.0, WIDTH) * 0.5,
        )
    }

    pub fn apply_acc(&mut self, v: Vec3) {
        self.vel += v;
    }

    pub fn update(&mut self, t_delta: f32, input: &InputState, world: &World) {
        // Key binds
        let key_a = Key::Character("a".into());
        let key_w = Key::Character("w".into());
        let key_s = Key::Character("s".into());
        let key_d = Key::Character("d".into());
        let key_z = Key::Character("z".into());
        let key_space = Key::Named(NamedKey::Space);
        let key_shift = Key::Named(NamedKey::Shift);

        let dx = self.rot.y.to_radians().sin() * self.speed;
        let dz = self.rot.y.to_radians().cos() * self.speed;

        if input.cursor_delta != Vec2::ZERO {
            self.handle_cursor_movement(t_delta, input.cursor_delta);
        }

        if self.flying {
            self.vel.y = 0.0;
        }
        if !self.flying {
            self.apply_acc(vec3(0.0, GRAVITY, 0.0));
        }
        self.vel *= 0.96;

        let mut frame_vel = self.vel;

        if input.key_pressed(&key_z) {
            self.flying = !self.flying;
            if self.flying {
                self.vel = Vec3::ZERO;
                return;
            }
        }

        if input.key_down(&key_w) {
            frame_vel.x += -dx;
            frame_vel.z += -dz;
        }
        if input.key_down(&key_s) {
            frame_vel.x += dx;
            frame_vel.z += dz;
        }
        if input.key_down(&key_d) {
            frame_vel.x += dz;
            frame_vel.z += -dx;
        }
        if input.key_down(&key_a) {
            frame_vel.x += -dz;
            frame_vel.z += dx;
        }
        if self.flying {
            if input.key_down(&key_space) {
                frame_vel.y += self.speed;
            }
            if input.key_down(&key_shift) {
                frame_vel.y += -self.speed;
            }
        } else {
            if input.key_down(&key_space) && self.on_ground {
                self.vel.y = 0.6;
                self.on_ground = false;
                frame_vel.y = 0.6;
            }
        }
        self.attempt_movement(world, frame_vel * t_delta);
    }

    pub fn eye_pos(&self) -> Vec3 {
        self.pos + vec3(0.0, 3.6, 0.0)
    }

    pub fn create_view_mat(&self) -> Mat4 {
        Mat4::from_translation(self.eye_pos())
            * Mat4::from_rotation_x(self.rot.x.to_radians())
            * Mat4::from_rotation_y(-self.rot.y.to_radians())
            * Mat4::from_rotation_z(self.rot.z.to_radians())
    }
    pub fn create_inv_view_mat(&self) -> Mat4 {
        Mat4::from_rotation_x(self.rot.x.to_radians())
            * Mat4::from_rotation_y(-self.rot.y.to_radians())
            * Mat4::from_rotation_z(self.rot.z.to_radians())
            * Mat4::from_translation(-self.eye_pos())
    }

    pub fn create_proj_mat(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect, 0.001, 1000.0)
    }

    pub fn create_cam_data(&self, proj_size: Vec2) -> CamData {
        let inv_view_mat = self.create_view_mat();
        let inv_proj_mat = self.create_proj_mat(proj_size.x / proj_size.y).inverse();

        CamData {
            pos: self.eye_pos(),
            inv_view_mat,
            inv_proj_mat,
            proj_size: vec2(proj_size.x, proj_size.y),
            ..Default::default()
        }
    }

    fn attempt_movement(&mut self, world: &World, mv: Vec3) {
        if self.flying {
            self.pos += mv;
            return;
        }

        struct ClippedMovement {
            result: Vec3,
            eq: BVec3,
        }

        let clip_movement = |world: &World, bbox: Aabb, mv: Vec3| -> ClippedMovement {
            // let world_bboxs = world.get_collisions_w(&bbox.expand(mv));

            // let mut result = mv;
            // for world_bbox in &world_bboxs {
            //     result.y = world_bbox.clip_y_collide(&bbox, result.y);
            //     result.x = world_bbox.clip_x_collide(&bbox, result.x);
            //     result.z = world_bbox.clip_z_collide(&bbox, result.z);
            // }
            // ClippedMovement {
            //     result,
            //     eq: result.cmpeq(mv),
            // }
            todo!()
        };
        let mut bbox = self.create_aabb();

        let ClippedMovement {
            result: mv_clipped,
            eq,
        } = clip_movement(world, bbox, mv);

        self.vel *= vec3(eq.x as i32 as f32, eq.y as i32 as f32, eq.z as i32 as f32);

        if !eq.x || !eq.z {
            // if we've been stopped in the X or Z direction,
            // test if we would be able to move forward if we were higher up.
            bbox.translate(vec3(0.0, 1.1, 0.0));

            let mut up_mv_clipped = clip_movement(world, bbox, mv);
            up_mv_clipped.result.y = 0.0;

            // if you can move furthur in any directrion when one space higher, then we should jump
            if up_mv_clipped.result.abs().cmpgt(mv_clipped.abs()).any() {
                self.pos += vec3(0.0, 1.1, 0.0);
            }
        }

        self.on_ground = self.vel.y == 0.0 && mv.y < 0.0;
        self.pos += mv_clipped;
    }

    pub fn facing(&self) -> Vec3 {
        axis_rot_to_ray(vec3(
            self.rot.x.to_radians(),
            self.rot.y.to_radians(),
            self.rot.z.to_radians(),
        ))
    }

    pub fn cast_ray(&self, world: &World) -> Option<HitResult> {
        use std::ops::Not;

        cast_ray(
            self.eye_pos(),
            axis_rot_to_ray(vec3(
                self.rot.x.to_radians(),
                self.rot.y.to_radians(),
                self.rot.z.to_radians(),
            )),
            100.0,
            |pos| {
                world
                    .get_voxel(pos)
                    .map(Voxel::is_empty)
                    .map(bool::not)
                    .unwrap_or(false)
            },
        )
    }
}
