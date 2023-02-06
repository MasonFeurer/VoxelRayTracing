use crate::aabb::Aabb;
use crate::cam::Cam;
use crate::input::{InputState, Key};
use crate::math::{HitResult, Mat4, Vec2f, Vec2u, Vec3f};
use crate::world::World;
use crate::{vec2f, vec3f};

const GRAVITY: f32 = -0.026;

#[derive(Clone)]
pub struct Player {
    pub flying: bool,
    pub pos: Vec3f,
    // in degrees
    pub rot: Vec3f,
    pub fov: f32,
    pub vel: Vec3f,
    pub on_ground: bool,
    pub speed: f32,
    pub aabb: Aabb,
}
impl Player {
    pub fn new(pos: Vec3f) -> Self {
        Self {
            flying: false,
            pos,
            rot: vec3f!(0.0),
            fov: 70.0,
            vel: vec3f!(0.0),
            on_ground: false,
            speed: 0.3,
            aabb: Self::create_aabb(pos),
        }
    }

    pub fn inv_proj_mat(&self, view_size: Vec2u) -> Mat4 {
        Mat4::projection(
            self.fov.to_radians(),
            (view_size.x as f32, view_size.y as f32),
            0.001,
            1000.0, // near and far, not sure if these even matter
        )
        .inverse()
        .unwrap()
    }

    pub fn handle_cursor_movement(&mut self, t_delta: f32, delta: Vec2f) {
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

    pub fn acc(&mut self, v: Vec3f) {
        self.vel += v;
    }

    pub fn set_pos(&mut self, pos: Vec3f) {
        self.pos = pos;
        self.aabb = Self::create_aabb(pos);
    }

    pub fn update(&mut self, t_delta: f32, input: &InputState, world: &World) {
        let dx = self.rot.y.to_radians().sin() * self.speed;
        let dz = self.rot.y.to_radians().cos() * self.speed;

        if input.cursor_delta != vec2f!(0.0) {
            self.handle_cursor_movement(t_delta, input.cursor_delta);
        }

        if self.on_ground || self.flying {
            self.vel.y = 0.0;
        } else {
            self.acc(vec3f!(0.0, GRAVITY * t_delta, 0.0));
        }
        self.vel *= 0.96;

        let mut frame_vel = self.vel;

        if input.key_pressed(Key::Z) {
            self.flying = !self.flying;
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
                self.vel.y = 0.4;
            }
        }
        self.attempt_movement(world, frame_vel * t_delta);
    }

    pub fn cam(&self) -> Cam {
        Cam {
            pos: self.pos + vec3f!(0.0, 1.6, 0.0),
            rot: self.rot,
        }
    }

    pub fn attempt_movement(&mut self, world: &World, mut a: Vec3f) {
        if self.flying {
            self.pos += a;
            self.aabb.translate(a);
            return;
        }

        let a_orig = a;

        let aabbs = world.get_collisions_w(&self.aabb.expand(a));

        for aabb in &aabbs {
            a.y = aabb.clip_y_collide(&self.aabb, a.y);
            a.x = aabb.clip_x_collide(&self.aabb, a.x);
            a.z = aabb.clip_z_collide(&self.aabb, a.z);
        }

        self.on_ground = a.y == 0.0 && a_orig.y < 0.0;

        if a_orig.x != a.x {
            self.vel.x = 0.0
        }
        if a_orig.y != a.y {
            self.vel.y = 0.0
        }
        if a_orig.z != a.z {
            self.vel.z = 0.0
        }

        self.pos += a;
        self.aabb.translate(a);
    }

    pub fn create_aabb(pos: Vec3f) -> Aabb {
        const WIDTH: f32 = 0.6;
        const HEIGHT: f32 = 1.8;

        Aabb::new(
            pos - vec3f!(WIDTH, 0.0, WIDTH) * 0.5,
            pos + vec3f!(WIDTH, HEIGHT * 2.0, WIDTH) * 0.5,
        )
    }

    pub fn cast_ray(&self, world: &World) -> Option<HitResult> {
        self.cam()
            .cast_ray(100.0, |pos| match world.get_voxel(pos) {
                Some(voxel) => voxel.is_solid(),
                None => false,
            })
    }
}
