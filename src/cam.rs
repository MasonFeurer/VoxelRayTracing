use crate::input::{InputState, Key};
use crate::matrices::Mat4;
use crate::vectors::{Vec2, Vec3};

/// Takes a rotation (the rotation around the X, Y, and Z axis), and
/// creates a normalized vector ray in the facing direction.<p>
/// the rotation values should be in radians (0..TAU)
pub fn axis_rot_to_ray(rot: Vec3<f32>) -> Vec3<f32> {
    // the Z rotation doesn't effect the ray
    // the Y rotation effects the ray's X and Z
    // the X rotation effects the ray's X, Y, and Z

    // the ray's X, Z is along the edge of a circle, cutting through the y-axis, radius R
    // R goes follows 0..1..0, and is derived from the X rotation (aka the vertical tilt)
    // the ray's Y is also derived from the X rotation

    // radius of Y axis cross-sections
    let r = rot.x.cos();
    let x = r * -rot.y.sin();
    let z = r * -rot.y.cos();
    let y = -rot.x.sin();
    Vec3 { x, y, z }
}

#[derive(Clone)]
pub struct Cam {
    pub pos: Vec3<f32>,
    // in degrees
    pub rot: Vec3<f32>,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}
impl Cam {
    pub fn new() -> Self {
        Self {
            pos: Vec3::new(20.0, 100.0, 20.0),
            rot: Vec3::new(0.0, 0.0, 0.0),
            fov: 70.0,
            near: 0.001,
            far: 1000.0,
        }
    }

    #[inline]
    pub fn dir(&self) -> Vec3<f32> {
        axis_rot_to_ray(self.rot.map(|e| e.to_radians()))
    }

    pub fn inv_proj_mat(&self, view_size: Vec2<u32>) -> Mat4 {
        Mat4::projection(
            self.fov.to_radians(),
            (view_size.x as f32, view_size.y as f32),
            self.near,
            self.far,
        )
        .inverse()
        .unwrap()
    }
    pub fn inv_view_mat(&self) -> Mat4 {
        assert_eq!(self.rot.z, 0.0);
        Mat4::view(self.pos, self.rot.map(f32::to_radians))
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

    pub fn update(&mut self, t_delta: f32, input: &InputState) {
        let dx = self.rot.y.to_radians().sin();
        let dz = self.rot.y.to_radians().cos();

        if input.cursor_delta != Vec2::all(0.0) {
            self.handle_cursor_movement(t_delta, input.cursor_delta);
        }
        let mut movement = Vec3::all(0.0);

        if input.key_is_pressed(Key::W) {
            movement.x += -dx;
            movement.z += -dz;
        }
        if input.key_is_pressed(Key::S) {
            movement.x += dx;
            movement.z += dz;
        }
        if input.key_is_pressed(Key::D) {
            movement.x += dz;
            movement.z += -dx;
        }
        if input.key_is_pressed(Key::A) {
            movement.x += -dz;
            movement.z += dx;
        }

        if input.key_is_pressed(Key::Space) {
            movement.y += 0.5;
        }
        if input.key_is_pressed(Key::LShift) {
            movement.y += -0.5;
        }
        self.pos += movement * t_delta * 0.5;
    }
}

#[derive(Clone)]
pub struct HitResult {
    pub pos: Vec3<i64>,
    pub face: Vec3<i64>,
}

pub fn cast_ray(
    start: Vec3<f32>,
    dir: Vec3<f32>,
    max_dist: f32,
    collides: impl Fn(Vec3<i64>) -> bool,
) -> Option<HitResult> {
    // -- DDA algorithm --

    // length of a line in same direction as the ray,
    // that travels 1 unit in the X, Y, Z
    let unit_step_size = Vec3::new(
        (1.0 + (dir.y / dir.x) * (dir.y / dir.x) + (dir.z / dir.x) * (dir.z / dir.x)).sqrt(),
        (1.0 + (dir.x / dir.y) * (dir.x / dir.y) + (dir.z / dir.y) * (dir.z / dir.y)).sqrt(),
        (1.0 + (dir.x / dir.z) * (dir.x / dir.z) + (dir.y / dir.z) * (dir.y / dir.z)).sqrt(),
    );

    let mut map_check = start.map(|e| e.floor() as i64);

    let (step, mut ray_len1d): (Vec3<f32>, Vec3<f32>) = {
        let (step_x, ray_len_x) = {
            if dir.x < 0.0 {
                (-1.0, (start.x - map_check.x as f32) * unit_step_size.x)
            } else {
                (1.0, ((map_check.x + 1) as f32 - start.x) * unit_step_size.x)
            }
        };
        let (step_y, ray_len_y) = {
            if dir.y < 0.0 {
                (-1.0, (start.y - map_check.y as f32) * unit_step_size.y)
            } else {
                (1.0, ((map_check.y + 1) as f32 - start.y) * unit_step_size.y)
            }
        };
        let (step_z, ray_len_z) = {
            if dir.z < 0.0 {
                (-1.0, (start.z - map_check.z as f32) * unit_step_size.z)
            } else {
                (1.0, ((map_check.z + 1) as f32 - start.z) * unit_step_size.z)
            }
        };
        (
            Vec3::new(step_x, step_y, step_z),
            Vec3::new(ray_len_x, ray_len_y, ray_len_z),
        )
    };
    let mut dist: f32 = 0.0;
    let mut prev_map_check;

    while dist < max_dist {
        prev_map_check = map_check;
        // walk
        if ray_len1d.x < ray_len1d.y && ray_len1d.x < ray_len1d.z {
            map_check.x += step.x as i64;
            dist = ray_len1d.x;
            ray_len1d.x += unit_step_size.x;
        } else if ray_len1d.z < ray_len1d.x && ray_len1d.z < ray_len1d.y {
            map_check.z += step.z as i64;
            dist = ray_len1d.z;
            ray_len1d.z += unit_step_size.z;
        } else {
            map_check.y += step.y as i64;
            dist = ray_len1d.y;
            ray_len1d.y += unit_step_size.y;
        }
        // check
        if collides(map_check) {
            return Some(HitResult {
                pos: map_check,
                face: prev_map_check - map_check,
            });
        }
    }
    None
}
