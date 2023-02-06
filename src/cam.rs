use crate::math::{axis_rot_to_ray, cast_ray, HitResult, Mat4, Vec3f, Vec3i};

#[derive(Clone)]
pub struct Cam {
    pub pos: Vec3f,
    // in degrees
    pub rot: Vec3f,
}
impl Cam {
    pub fn view_mat(&self) -> Mat4 {
        Mat4::view(self.pos, self.rot.to_radians())
    }

    pub fn cast_ray(&self, max_dist: f32, f: impl Fn(Vec3i) -> bool) -> Option<HitResult> {
        let dir = axis_rot_to_ray(self.rot.to_radians());
        cast_ray(self.pos, dir, max_dist, f)
    }
}
