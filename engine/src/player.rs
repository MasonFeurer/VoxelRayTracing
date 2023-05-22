use crate::gpu::shaders::CamData;
use crate::input::{InputState, Key};
use crate::math::aabb::Aabb;
use crate::math::dda::{axis_rot_to_ray, cast_ray, HitResult};
use crate::world::World;
use glam::{Mat4, Vec2, Vec3};

const GRAVITY: f32 = -0.020;

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
        const HEIGHT: f32 = 1.8;

        Aabb::new(
            self.pos - Vec3::new(WIDTH, 0.0, WIDTH) * 0.5,
            self.pos + Vec3::new(WIDTH, HEIGHT * 2.0, WIDTH) * 0.5,
        )
    }

    pub fn apply_acc(&mut self, v: Vec3) {
        self.vel += v;
    }

    pub fn update(&mut self, t_delta: f32, input: &InputState, world: &World) {
        let dx = self.rot.y.to_radians().sin() * self.speed;
        let dz = self.rot.y.to_radians().cos() * self.speed;

        if input.cursor_delta != Vec2::ZERO {
            self.handle_cursor_movement(t_delta, input.cursor_delta);
        }

        if self.flying {
            self.vel.y = 0.0;
        }
        if !self.flying {
            self.apply_acc(Vec3::new(0.0, GRAVITY, 0.0));
        }
        self.vel *= 0.96;

        let mut frame_vel = self.vel;

        if input.key_pressed(Key::Z) {
            self.flying = !self.flying;
            if self.flying {
                self.vel = Vec3::ZERO;
                return;
            }
        }

        if input.key_down(Key::W) {
            frame_vel.x += -dx;
            frame_vel.z += -dz;
        }
        if input.key_down(Key::S) {
            frame_vel.x += dx;
            frame_vel.z += dz;
        }
        if input.key_down(Key::D) {
            frame_vel.x += dz;
            frame_vel.z += -dx;
        }
        if input.key_down(Key::A) {
            frame_vel.x += -dz;
            frame_vel.z += dx;
        }
        if self.flying {
            if input.key_down(Key::Space) {
                frame_vel.y += self.speed;
            }
            if input.key_down(Key::LShift) {
                frame_vel.y += -self.speed;
            }
        } else {
            if input.key_down(Key::Space) && self.on_ground {
                self.vel.y = 0.3;
                self.on_ground = false;
                frame_vel.y = 0.3;
            }
        }
        self.attempt_movement(world, frame_vel * t_delta);
    }

    pub fn eye_pos(&self) -> Vec3 {
        self.pos + Vec3::new(0.0, 1.6, 0.0)
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
            proj_size: Vec2::new(proj_size.x, proj_size.y),
            ..Default::default()
        }
    }

    fn attempt_movement(&mut self, world: &World, mut a: Vec3) {
        if self.flying {
            self.pos += a;
            return;
        }

        let a_orig = a;

        let self_aabb = self.create_aabb();
        let aabbs = world.get_collisions_w(&self_aabb.expand(a));

        for aabb in &aabbs {
            a.y = aabb.clip_y_collide(&self_aabb, a.y);
            a.x = aabb.clip_x_collide(&self_aabb, a.x);
            a.z = aabb.clip_z_collide(&self_aabb, a.z);
        }

        if a_orig.x != a.x {
            self.vel.x = 0.0
        }
        if a_orig.y != a.y {
            self.vel.y = 0.0
        }
        if a_orig.z != a.z {
            self.vel.z = 0.0
        }
        self.on_ground = self.vel.y == 0.0 && a_orig.y < 0.0;
        self.pos += a;
    }

    pub fn cast_ray(&self, world: &World) -> Option<HitResult> {
        cast_ray(
            self.eye_pos(),
            axis_rot_to_ray(Vec3::new(
                self.rot.x.to_radians(),
                self.rot.y.to_radians(),
                self.rot.z.to_radians(),
            )),
            100.0,
            |pos| match world.get_voxel(pos) {
                Some(voxel) => voxel.is_solid(),
                None => false,
            },
        )
    }
}
