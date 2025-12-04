use crate::common::math::axis_rot_to_ray;
use crate::common::math::Aabb;
use glam::{vec3, Vec2, Vec3};

const GRAVITY: f32 = -0.050;

#[derive(Default)]
pub struct PlayerInput {
    pub cursor_movement: Vec2,
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub backward: bool,
    pub jump: bool,
    pub crouch: bool,
    pub toggle_fly: bool,
}

#[derive(Default)]
pub struct PlayerMovement {
    pub new_cam: Vec3,
    pub cam_moved: bool,
    pub new_vel: Vec3,
    pub frame_vel: Vec3,
    pub flying: bool,
}

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

    pub height: f32,
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

            height: 4.0,
        }
    }

    pub fn create_aabb(&self) -> Aabb {
        const WIDTH: f32 = 0.9;

        Aabb::new(
            self.pos - vec3(WIDTH * 0.5, 0.0, WIDTH * 0.5),
            self.pos + vec3(WIDTH * 0.5, self.height, WIDTH * 0.5),
        )
    }

    pub fn process_input(&self, t_delta: f32, input: &PlayerInput) -> PlayerMovement {
        let dx = self.rot.y.to_radians().sin() * self.speed;
        let dz = self.rot.y.to_radians().cos() * self.speed;

        let mut result = PlayerMovement::default();

        // ------- Camera -------
        {
            const SENSITIVITY: f32 = 0.3;
            let delta = input.cursor_movement * t_delta;

            // in model space, the camera is looking negative along the Z axis, so
            // moving the cursor up/down corresponds to rotation about the X axis
            result.new_cam.x = (self.rot.x + SENSITIVITY * delta.y).clamp(-90.0, 90.0);

            // moving the cursor left/right corresponds to rotation about the Y axis
            result.new_cam.y = self.rot.y - SENSITIVITY * delta.x;

            // the camera does not rotate about the Z axis. That would be like tilting your head
            result.cam_moved = self.rot != result.new_cam;
        }
        result.new_vel = self.vel;

        if self.flying {
            result.new_vel.y = 0.0;
        } else {
            result.new_vel.y += GRAVITY;
        }
        result.new_vel *= 0.95;

        let mut frame_vel = result.new_vel;

        result.flying = self.flying;
        if input.toggle_fly {
            result.flying = !result.flying;
            if result.flying {
                result.new_vel = Vec3::ZERO;
                return result;
            }
        }

        if input.forward {
            frame_vel.x += -dx;
            frame_vel.z += -dz;
        }
        if input.backward {
            frame_vel.x += dx;
            frame_vel.z += dz;
        }
        if input.right {
            frame_vel.x += dz;
            frame_vel.z += -dx;
        }
        if input.left {
            frame_vel.x += -dz;
            frame_vel.z += dx;
        }
        if self.flying {
            if input.jump {
                frame_vel.y += self.speed;
            }
            if input.crouch {
                frame_vel.y += -self.speed;
            }
        } else {
            if input.jump && self.on_ground {
                result.new_vel.y = 0.6;
                frame_vel.y = 0.6;
            }
        }
        result.frame_vel = frame_vel * t_delta;
        result
    }

    pub fn update(&mut self, input: &PlayerMovement, world: impl Fn(&Aabb) -> Vec<Aabb>) {
        self.vel = input.new_vel;
        self.rot = input.new_cam;
        self.flying = input.flying;

        if self.flying {
            self.pos += input.frame_vel;
        } else {
            let vel_clipped = clip_aabb_movement(self.create_aabb(), input.frame_vel, world, true);
            self.pos += vel_clipped;
            self.on_ground = vel_clipped.y.abs() < 0.001 && input.frame_vel.y < 0.001;
        }
    }

    pub fn eye_pos(&self) -> Vec3 {
        self.pos + vec3(0.0, self.height, 0.0)
    }

    pub fn facing(&self) -> Vec3 {
        axis_rot_to_ray(vec3(
            self.rot.x.to_radians(),
            self.rot.y.to_radians(),
            self.rot.z.to_radians(),
        ))
    }
}

pub fn clip_aabb_movement(
    mut bbox: Aabb,
    mv: Vec3,
    world: impl Fn(&Aabb) -> Vec<Aabb>,
    autojump: bool,
) -> Vec3 {
    let world_bboxs = world(&bbox.expand(mv));

    let mut mv_clipped = mv;
    for world_bbox in &world_bboxs {
        mv_clipped.y = world_bbox.clip_y_collide(&bbox, mv_clipped.y);
        mv_clipped.x = world_bbox.clip_x_collide(&bbox, mv_clipped.x);
        mv_clipped.z = world_bbox.clip_z_collide(&bbox, mv_clipped.z);
    }
    let eq = mv_clipped.cmpeq(mv);

    // cancel entities velocity in any direction that was stopped by the world
    // self.vel *= vec3(eq.x as i32 as f32, eq.y as i32 as f32, eq.z as i32 as f32);

    if autojump && (!eq.x || !eq.z) {
        // For autojump: if we've been stopped in the X or Z direction,
        // test if we would be able to move forward if we were higher up.
        bbox = bbox.translate(vec3(0.0, 1.1, 0.0));

        let world_bboxs = world(&bbox.expand(mv));

        let mut jmp_clipped = mv;
        for world_bbox in &world_bboxs {
            jmp_clipped.y = world_bbox.clip_y_collide(&bbox, jmp_clipped.y);
            jmp_clipped.x = world_bbox.clip_x_collide(&bbox, jmp_clipped.x);
            jmp_clipped.z = world_bbox.clip_z_collide(&bbox, jmp_clipped.z);
        }
        jmp_clipped.y = 0.0;

        // if you can move further in any direction when one space higher, then we should jump
        if jmp_clipped.abs().cmpgt(mv_clipped.abs()).any() {
            mv_clipped.y += 1.0;
            mv_clipped.x = jmp_clipped.x;
            mv_clipped.z = jmp_clipped.z;
        }
    }
    mv_clipped
}
