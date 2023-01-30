use crate::matrices::Mat4;
use crate::vectors::Vec3;

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
}
impl Cam {
    pub fn view_mat(&self) -> Mat4 {
        Mat4::view(self.pos, self.rot.map(f32::to_radians))
    }

    pub fn cast_ray(
        &self,
        max_dist: f32,
        collides: impl Fn(Vec3<i32>) -> bool,
    ) -> Option<HitResult> {
        let dir = axis_rot_to_ray(self.rot.map(f32::to_radians));
        cast_ray(self.pos, dir, max_dist, collides)
    }
}

#[derive(Clone, Copy)]
pub struct HitResult {
    pub pos: Vec3<i32>,
    pub face: Vec3<i32>,
}

pub fn cast_ray(
    start: Vec3<f32>,
    dir: Vec3<f32>,
    max_dist: f32,
    collides: impl Fn(Vec3<i32>) -> bool,
) -> Option<HitResult> {
    // -- DDA algorithm --

    // length of a line in same direction as the ray,
    // that travels 1 unit in the X, Y, Z
    let unit_step_size = Vec3::new(
        (1.0 + (dir.y / dir.x) * (dir.y / dir.x) + (dir.z / dir.x) * (dir.z / dir.x)).sqrt(),
        (1.0 + (dir.x / dir.y) * (dir.x / dir.y) + (dir.z / dir.y) * (dir.z / dir.y)).sqrt(),
        (1.0 + (dir.x / dir.z) * (dir.x / dir.z) + (dir.y / dir.z) * (dir.y / dir.z)).sqrt(),
    );

    let mut map_check = start.map(|e| e.floor() as i32);

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
            map_check.x += step.x as i32;
            dist = ray_len1d.x;
            ray_len1d.x += unit_step_size.x;
        } else if ray_len1d.z < ray_len1d.x && ray_len1d.z < ray_len1d.y {
            map_check.z += step.z as i32;
            dist = ray_len1d.z;
            ray_len1d.z += unit_step_size.z;
        } else {
            map_check.y += step.y as i32;
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
