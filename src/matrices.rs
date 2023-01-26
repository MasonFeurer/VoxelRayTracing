use crate::vectors::{Vec3, Vec4, VecMath};

#[derive(Clone)]
pub struct Mat4(pub [f32; 16]);
impl Mat4 {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self([0.0; 16])
    }
    #[rustfmt::skip]
    #[inline(always)]
    pub const fn identity() -> Self {
        Self([
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    #[inline(always)]
    pub const fn get(&self, col: usize, row: usize) -> f32 {
        self.0[row + col * 4]
    }
    #[inline(always)]
    pub fn set(&mut self, col: usize, row: usize, value: f32) {
        self.0[row + col * 4] = value;
    }

    #[inline]
    pub fn get_row(&self, row: usize) -> Vec4<f32> {
        Vec4 {
            x: self.get(0, row),
            y: self.get(1, row),
            z: self.get(2, row),
            w: self.get(3, row),
        }
    }
    #[inline]
    pub fn get_col(&self, col: usize) -> Vec4<f32> {
        Vec4 {
            x: self.get(col, 0),
            y: self.get(col, 1),
            z: self.get(col, 2),
            w: self.get(col, 3),
        }
    }

    #[inline(always)]
    pub fn inverse(self) -> Option<Self> {
        invert_matrix(self.0).map(|m| Self(m))
    }

    /// Creates a 4x4 matrix that rotates a `Pos3` about the X axis.
    /// Expects angle `a` to be in radians.
    pub fn x_rotation(a: f32) -> Self {
        let (s, c) = (a.sin(), a.cos());
        let mut out = Self::identity();
        out.set(1, 1, c);
        out.set(2, 2, c);
        out.set(2, 1, s);
        out.set(1, 2, -s);
        out
    }
    /// Creates a 4x4 matrix that rotates a `Pos3` about the Y axis.
    /// Expects angle `a` to be in radians.
    pub fn y_rotation(a: f32) -> Self {
        let (s, c) = (a.sin(), a.cos());
        let mut out = Self::identity();
        out.set(0, 0, c);
        out.set(2, 0, s);
        out.set(0, 2, -s);
        out.set(2, 2, c);
        out
    }
    /// Creates a 4x4 matrix that rotates a `Pos3` about the Z axis.
    /// Expects angle `a` to be in radians.
    pub fn z_rotation(a: f32) -> Self {
        let (s, c) = (a.sin(), a.cos());
        let mut out = Self::identity();
        out.set(0, 0, c);
        out.set(1, 0, s);
        out.set(0, 1, s);
        out.set(1, 1, c);
        out
    }

    /// Creates a 4x4 matrix that rotates a `Pos3` by `rot`, translates by `trans`, and scales by `scale`.
    /// Expects `fov` elements to be in radians.
    pub fn transformation(trans: Vec3<f32>, rot: Vec3<f32>, scale: Vec3<f32>) -> Self {
        Self::translation(trans)
            * Self::x_rotation(rot.x)
            * Self::y_rotation(rot.y)
            * Self::z_rotation(rot.z)
            * Self::scaling(scale)
    }

    /// Creates a 4x4 matrix that projects a `Pos3` in world space, to screen space.
    /// expects `fov` to be in radians.
    pub fn projection(fov: f32, view_size: (f32, f32), near: f32, far: f32) -> Self {
        let aspect = view_size.0 / view_size.1;
        let x_scale = 1.0 / (fov / 2.0).tan();
        let y_scale = x_scale * aspect;
        let range = far - near;

        let mut out = Self::empty();
        out.set(0, 0, x_scale);
        out.set(1, 1, y_scale);
        out.set(2, 2, -((far + near) / range));
        out.set(3, 2, -1.0);
        out.set(2, 3, -((2.0 * near * far) / range));
        out
    }

    /// Creates a 4x4 matrix that rotates a `Pos3` by `rot`, and translates it by the negative of `pos`.
    /// Expects `rot` elements to be in radians.
    pub fn view(pos: Vec3<f32>, rot: Vec3<f32>) -> Self {
        Self::x_rotation(rot.x)
            * Self::y_rotation(rot.y)
            * Self::z_rotation(rot.z)
            * Self::translation(pos * -1.0)
    }

    /// Creates a 4x4 matrix that translates a `Pos3` by `t`.
    pub fn translation(t: Vec3<f32>) -> Self {
        let mut out = Self::identity();
        out.set(0, 3, t.x);
        out.set(1, 3, t.y);
        out.set(2, 3, t.z);
        out
    }

    /// Creates a 4x4 matrix that scales a `Pos3` by `scale`
    pub fn scaling(scale: Vec3<f32>) -> Self {
        let mut out = Self::identity();
        out.set(0, 0, scale.x);
        out.set(1, 1, scale.y);
        out.set(2, 2, scale.z);
        out
    }
}
impl std::ops::Mul for Mat4 {
    type Output = Self;
    fn mul(self, other: Self) -> Self::Output {
        let mut out = Self::empty();
        out.set(0, 0, self.get_col(0).dot(other.get_row(0)) as f32);
        out.set(1, 0, self.get_col(1).dot(other.get_row(0)) as f32);
        out.set(2, 0, self.get_col(2).dot(other.get_row(0)) as f32);
        out.set(3, 0, self.get_col(3).dot(other.get_row(0)) as f32);

        out.set(0, 1, self.get_col(0).dot(other.get_row(1)) as f32);
        out.set(1, 1, self.get_col(1).dot(other.get_row(1)) as f32);
        out.set(2, 1, self.get_col(2).dot(other.get_row(1)) as f32);
        out.set(3, 1, self.get_col(3).dot(other.get_row(1)) as f32);

        out.set(0, 2, self.get_col(0).dot(other.get_row(2)) as f32);
        out.set(1, 2, self.get_col(1).dot(other.get_row(2)) as f32);
        out.set(2, 2, self.get_col(2).dot(other.get_row(2)) as f32);
        out.set(3, 2, self.get_col(3).dot(other.get_row(2)) as f32);

        out.set(0, 3, self.get_col(0).dot(other.get_row(3)) as f32);
        out.set(1, 3, self.get_col(1).dot(other.get_row(3)) as f32);
        out.set(2, 3, self.get_col(2).dot(other.get_row(3)) as f32);
        out.set(3, 3, self.get_col(3).dot(other.get_row(3)) as f32);
        out
    }
}

#[rustfmt::skip]
fn invert_matrix(m: [f32; 16]) -> Option<[f32; 16]> {
	let mut inv = [0.0; 16];

    inv[0] = m[5]  * m[10] * m[15] - 
             m[5]  * m[11] * m[14] - 
             m[9]  * m[6]  * m[15] + 
             m[9]  * m[7]  * m[14] +
             m[13] * m[6]  * m[11] - 
             m[13] * m[7]  * m[10];

    inv[4] = -m[4]  * m[10] * m[15] + 
              m[4]  * m[11] * m[14] + 
              m[8]  * m[6]  * m[15] - 
              m[8]  * m[7]  * m[14] - 
              m[12] * m[6]  * m[11] + 
              m[12] * m[7]  * m[10];

    inv[8] = m[4]  * m[9] * m[15] - 
             m[4]  * m[11] * m[13] - 
             m[8]  * m[5] * m[15] + 
             m[8]  * m[7] * m[13] + 
             m[12] * m[5] * m[11] - 
             m[12] * m[7] * m[9];

    inv[12] = -m[4]  * m[9] * m[14] + 
               m[4]  * m[10] * m[13] +
               m[8]  * m[5] * m[14] - 
               m[8]  * m[6] * m[13] - 
               m[12] * m[5] * m[10] + 
               m[12] * m[6] * m[9];

    inv[1] = -m[1]  * m[10] * m[15] + 
              m[1]  * m[11] * m[14] + 
              m[9]  * m[2] * m[15] - 
              m[9]  * m[3] * m[14] - 
              m[13] * m[2] * m[11] + 
              m[13] * m[3] * m[10];

    inv[5] = m[0]  * m[10] * m[15] - 
             m[0]  * m[11] * m[14] - 
             m[8]  * m[2] * m[15] + 
             m[8]  * m[3] * m[14] + 
             m[12] * m[2] * m[11] - 
             m[12] * m[3] * m[10];

    inv[9] = -m[0]  * m[9] * m[15] + 
              m[0]  * m[11] * m[13] + 
              m[8]  * m[1] * m[15] - 
              m[8]  * m[3] * m[13] - 
              m[12] * m[1] * m[11] + 
              m[12] * m[3] * m[9];

    inv[13] = m[0]  * m[9] * m[14] - 
              m[0]  * m[10] * m[13] - 
              m[8]  * m[1] * m[14] + 
              m[8]  * m[2] * m[13] + 
              m[12] * m[1] * m[10] - 
              m[12] * m[2] * m[9];

    inv[2] = m[1]  * m[6] * m[15] - 
             m[1]  * m[7] * m[14] - 
             m[5]  * m[2] * m[15] + 
             m[5]  * m[3] * m[14] + 
             m[13] * m[2] * m[7] - 
             m[13] * m[3] * m[6];

    inv[6] = -m[0]  * m[6] * m[15] + 
              m[0]  * m[7] * m[14] + 
              m[4]  * m[2] * m[15] - 
              m[4]  * m[3] * m[14] - 
              m[12] * m[2] * m[7] + 
              m[12] * m[3] * m[6];

    inv[10] = m[0]  * m[5] * m[15] - 
              m[0]  * m[7] * m[13] - 
              m[4]  * m[1] * m[15] + 
              m[4]  * m[3] * m[13] + 
              m[12] * m[1] * m[7] - 
              m[12] * m[3] * m[5];

    inv[14] = -m[0]  * m[5] * m[14] + 
               m[0]  * m[6] * m[13] + 
               m[4]  * m[1] * m[14] - 
               m[4]  * m[2] * m[13] - 
               m[12] * m[1] * m[6] + 
               m[12] * m[2] * m[5];

    inv[3] = -m[1] * m[6] * m[11] + 
              m[1] * m[7] * m[10] + 
              m[5] * m[2] * m[11] - 
              m[5] * m[3] * m[10] - 
              m[9] * m[2] * m[7] + 
              m[9] * m[3] * m[6];

    inv[7] = m[0] * m[6] * m[11] - 
             m[0] * m[7] * m[10] - 
             m[4] * m[2] * m[11] + 
             m[4] * m[3] * m[10] + 
             m[8] * m[2] * m[7] - 
             m[8] * m[3] * m[6];

    inv[11] = -m[0] * m[5] * m[11] + 
               m[0] * m[7] * m[9] + 
               m[4] * m[1] * m[11] - 
               m[4] * m[3] * m[9] - 
               m[8] * m[1] * m[7] + 
               m[8] * m[3] * m[5];

    inv[15] = m[0] * m[5] * m[10] - 
              m[0] * m[6] * m[9] - 
              m[4] * m[1] * m[10] + 
              m[4] * m[2] * m[9] + 
              m[8] * m[1] * m[6] - 
              m[8] * m[2] * m[5];

    let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
    if det == 0.0 { return None }
    let det = 1.0 / det;
    
    for v in &mut inv {
    	*v *= det;
    }
    Some(inv)
}
