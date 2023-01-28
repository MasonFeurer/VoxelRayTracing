use crate::aabb::Aabb;
use crate::cam::Cam;
use crate::input::{InputState, Key};
use crate::matrices::Mat4;
use crate::vectors::{Vec2, Vec3};
use crate::world::World;

const GRAVITY: f32 = -0.02;

#[derive(Clone)]
pub struct Player {
    pub pos: Vec3<f32>,
    // in degrees
    pub rot: Vec3<f32>,
    pub fov: f32,
    pub vel: Vec3<f32>,
    pub on_ground: bool,
    pub speed: f32,
    pub aabb: Aabb,
}
impl Player {
    pub fn new(pos: Vec3<f32>) -> Self {
        Self {
            pos,
            rot: Vec3::new(0.0, 0.0, 0.0),
            fov: 70.0,
            vel: Vec3::new(0.0, 0.0, 0.0),
            on_ground: false,
            speed: 0.1,
            aabb: Self::create_aabb(pos),
        }
    }

    pub fn inv_proj_mat(&self, view_size: Vec2<u32>) -> Mat4 {
        Mat4::projection(
            self.fov.to_radians(),
            (view_size.x as f32, view_size.y as f32),
            0.001,
            1000.0, // near and far, not sure if these even matter
        )
        .inverse()
        .unwrap()
    }

    pub fn handle_cursor_movement(&mut self, t_delta: f32, delta: Vec2<f32>) {
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

    pub fn acc(&mut self, v: Vec3<f32>) {
        self.vel += v;
    }

    pub fn set_pos(&mut self, pos: Vec3<f32>) {
        self.pos = pos;
        self.aabb = Self::create_aabb(pos);
    }

    pub fn update(&mut self, t_delta: f32, input: &InputState, world: &World) {
        let dx = self.rot.y.to_radians().sin() * self.speed;
        let dz = self.rot.y.to_radians().cos() * self.speed;

        if input.cursor_delta != Vec2::all(0.0) {
            self.handle_cursor_movement(t_delta, input.cursor_delta);
        }

        if self.on_ground {
            self.vel.y = 0.0;
        } else {
            self.acc(Vec3::new(0.0, GRAVITY * t_delta, 0.0));
        }
        self.vel *= 0.96;

        if input.key_down(Key::Space) && self.on_ground {
            self.vel.y = 0.3;
        }

        let mut frame_vel = self.vel;

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
        self.attempt_movement(world, frame_vel * t_delta);
    }

    pub fn cam(&self) -> Cam {
        Cam {
            pos: self.pos + Vec3::new(0.0, 1.6, 0.0),
            rot: self.rot,
        }
    }

    pub fn attempt_movement(&mut self, world: &World, mut a: Vec3<f32>) {
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

    pub fn create_aabb(pos: Vec3<f32>) -> Aabb {
        const WIDTH: f32 = 0.6;
        const HEIGHT: f32 = 1.8;

        Aabb::new(
            pos - Vec3::new(WIDTH, 0.0, WIDTH) * 0.5,
            pos + Vec3::new(WIDTH, HEIGHT * 2.0, WIDTH) * 0.5,
        )
    }
}
