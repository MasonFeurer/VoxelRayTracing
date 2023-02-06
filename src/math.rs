macro_rules! impl_vec_ops {
    ($name:ident{$($field:ident),*}:$ty:ty) => {
        impl std::ops::Add<$name> for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self { $($field: self.$field + rhs.$field),* }
            }
        }
        impl std::ops::Add<$ty> for $name {
            type Output = Self;
            fn add(self, rhs: $ty) -> Self {
                Self { $($field: self.$field + rhs),* }
            }
        }
        impl std::ops::Sub<$name> for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self { $($field: self.$field - rhs.$field),* }
            }
        }
        impl std::ops::Sub<$ty> for $name {
            type Output = Self;
            fn sub(self, rhs: $ty) -> Self {
                Self { $($field: self.$field - rhs),* }
            }
        }
        impl std::ops::Mul<$name> for $name {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self {
                Self { $($field: self.$field * rhs.$field),* }
            }
        }
        impl std::ops::Mul<$ty> for $name {
            type Output = Self;
            fn mul(self, rhs: $ty) -> Self {
                Self { $($field: self.$field * rhs),* }
            }
        }
        impl std::ops::Div<$name> for $name {
            type Output = Self;
            fn div(self, rhs: Self) -> Self {
                Self { $($field: self.$field / rhs.$field),* }
            }
        }
        impl std::ops::Div<$ty> for $name {
            type Output = Self;
            fn div(self, rhs: $ty) -> Self {
                Self { $($field: self.$field / rhs),* }
            }
        }
        impl std::ops::Rem<$name> for $name {
            type Output = Self;
            fn rem(self, rhs: Self) -> Self {
                Self { $($field: self.$field % rhs.$field),* }
            }
        }
        impl std::ops::Rem<$ty> for $name {
            type Output = Self;
            fn rem(self, rhs: $ty) -> Self {
                Self { $($field: self.$field % rhs),* }
            }
        }
        // impl std::ops::Neg for $name {
        //     type Output = Self;
        //     fn neg(self) -> Self {
        //         Self { $($field: -self.$field),* }
        //     }
        // }

        impl std::ops::AddAssign<$name> for $name {
            fn add_assign(&mut self, rhs: Self) {
                $(self.$field += rhs.$field;)*
            }
        }
        impl std::ops::AddAssign<$ty> for $name {
            fn add_assign(&mut self, rhs: $ty) {
                $(self.$field += rhs;)*
            }
        }
        impl std::ops::SubAssign<$name> for $name {
            fn sub_assign(&mut self, rhs: Self) {
                $(self.$field -= rhs.$field;)*
            }
        }
        impl std::ops::SubAssign<$ty> for $name {
            fn sub_assign(&mut self, rhs: $ty) {
                $(self.$field -= rhs;)*
            }
        }
        impl std::ops::MulAssign<$name> for $name {
            fn mul_assign(&mut self, rhs: Self) {
                $(self.$field *= rhs.$field;)*
            }
        }
        impl std::ops::MulAssign<$ty> for $name {
            fn mul_assign(&mut self, rhs: $ty) {
                $(self.$field *= rhs;)*
            }
        }
        impl std::ops::DivAssign<$name> for $name {
            fn div_assign(&mut self, rhs: Self) {
                $(self.$field /= rhs.$field;)*
            }
        }
        impl std::ops::DivAssign<$ty> for $name {
            fn div_assign(&mut self, rhs: $ty) {
                $(self.$field /= rhs;)*
            }
        }
        impl std::ops::RemAssign<$name> for $name {
            fn rem_assign(&mut self, rhs: Self) {
                $(self.$field %= rhs.$field;)*
            }
        }
        impl std::ops::RemAssign<$ty> for $name {
            fn rem_assign(&mut self, rhs: $ty) {
                $(self.$field %= rhs;)*
            }
        }
    }
}

macro_rules! define_vec2 {
    ($name:ident: $ty:ty) => {
        #[derive(Clone, Copy, Default, Debug, PartialEq, bytemuck::Zeroable, bytemuck::Pod)]
        #[repr(C)]
        pub struct $name {
            pub x: $ty,
            pub y: $ty,
        }
        impl From<[$ty; 2]> for $name {
            fn from(v: [$ty; 2]) -> Self {
                Self { x: v[0], y: v[1] }
            }
        }
        impl From<$name> for [$ty; 2] {
            fn from(v: $name) -> Self {
                [v.x, v.y]
            }
        }
        impl $name {
            #[inline(always)]
            pub const fn new(x: $ty, y: $ty) -> Self {
                Self { x, y }
            }

            #[inline(always)]
            pub fn map(&self, f: impl Fn($ty) -> $ty) -> Self {
                Self {
                    x: f(self.x),
                    y: f(self.y),
                }
            }
        }
        impl_vec_ops!($name { x, y }: $ty);
    };
}
macro_rules! define_vec3 {
    ($name:ident: $ty:ty) => {
        #[derive(Clone, Copy, Default, Debug, PartialEq, bytemuck::Zeroable, bytemuck::Pod)]
        #[repr(C)]
        pub struct $name {
            pub x: $ty,
            pub y: $ty,
            pub z: $ty,
        }
        impl From<[$ty; 3]> for $name {
            fn from(v: [$ty; 3]) -> Self {
                Self {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                }
            }
        }
        impl From<$name> for [$ty; 3] {
            fn from(v: $name) -> Self {
                [v.x, v.y, v.z]
            }
        }
        impl $name {
            #[inline(always)]
            pub const fn new(x: $ty, y: $ty, z: $ty) -> Self {
                Self { x, y, z }
            }

            #[inline(always)]
            pub fn map(&self, f: impl Fn($ty) -> $ty) -> Self {
                Self {
                    x: f(self.x),
                    y: f(self.y),
                    z: f(self.z),
                }
            }
        }
        impl_vec_ops!($name { x, y, z }: $ty);
    };
}

define_vec3!(Vec3f: f32);
define_vec3!(Vec3i: i32);
define_vec3!(Vec3u: u32);

define_vec2!(Vec2f: f32);
define_vec2!(Vec2i: i32);
define_vec2!(Vec2u: u32);

#[macro_export]
macro_rules! vec3f {
    ($v: expr) => {
        crate::math::Vec3f::new($v, $v, $v)
    };
    ($x: expr, $y: expr, $z: expr$(,)?) => {
        crate::math::Vec3f::new($x, $y, $z)
    };
}
#[macro_export]
macro_rules! vec3i {
    ($v: expr) => {
        crate::math::Vec3i::new($v, $v, $v)
    };
    ($x: expr, $y: expr, $z: expr$(,)?) => {
        crate::math::Vec3i::new($x, $y, $z)
    };
}
#[macro_export]
macro_rules! vec3u {
    ($v: expr) => {
        crate::math::Vec3u::new($x, $y, $z)
    };
    ($x: expr, $y: expr, $z: expr$(,)?) => {
        crate::math::Vec3u::new($x, $y, $z)
    };
}

#[macro_export]
macro_rules! vec2f {
    ($v: expr) => {
        crate::math::Vec2f::new($v, $v)
    };
    ($x: expr, $y: expr$(,)?) => {
        crate::math::Vec2f::new($x, $y)
    };
}
#[macro_export]
macro_rules! vec2i {
    ($v: expr) => {
        crate::math::Vec2i::new($v, $v)
    };
    ($x: expr, $y: expr$(,)?) => {
        crate::math::Vec2i::new($x, $y)
    };
}
#[macro_export]
macro_rules! vec2u {
    ($v: expr) => {
        crate::math::Vec2u::new($v, $v)
    };
    ($x: expr, $y: expr$(,)?) => {
        crate::math::Vec2u::new($x, $y)
    };
}

impl Vec3f {
    pub fn floor(self) -> Vec3i {
        Vec3i {
            x: self.x.floor() as i32,
            y: self.y.floor() as i32,
            z: self.z.floor() as i32,
        }
    }
    pub fn ceil(self) -> Vec3i {
        Vec3i {
            x: self.x.ceil() as i32,
            y: self.y.ceil() as i32,
            z: self.z.ceil() as i32,
        }
    }
    pub fn to_radians(self) -> Self {
        Self {
            x: self.x.to_radians(),
            y: self.y.to_radians(),
            z: self.z.to_radians(),
        }
    }
}

#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
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
    pub fn get_row(&self, row: usize) -> [f32; 4] {
        [
            self.get(0, row),
            self.get(1, row),
            self.get(2, row),
            self.get(3, row),
        ]
    }
    #[inline]
    pub fn get_col(&self, col: usize) -> [f32; 4] {
        [
            self.get(col, 0),
            self.get(col, 1),
            self.get(col, 2),
            self.get(col, 3),
        ]
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
    pub fn transformation(trans: Vec3f, rot: Vec3f, scale: Vec3f) -> Self {
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
    pub fn view(pos: Vec3f, rot: Vec3f) -> Self {
        Self::x_rotation(rot.x)
            * Self::y_rotation(rot.y)
            * Self::z_rotation(rot.z)
            * Self::translation(pos * -1.0)
    }

    /// Creates a 4x4 matrix that translates a `Pos3` by `t`.
    pub fn translation(t: Vec3f) -> Self {
        let mut out = Self::identity();
        out.set(0, 3, t.x);
        out.set(1, 3, t.y);
        out.set(2, 3, t.z);
        out
    }

    /// Creates a 4x4 matrix that scales a `Pos3` by `scale`
    pub fn scaling(scale: Vec3f) -> Self {
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

        let dot = |a: [f32; 4], b: [f32; 4]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];

        out.set(0, 0, dot(self.get_col(0), other.get_row(0)) as f32);
        out.set(1, 0, dot(self.get_col(1), other.get_row(0)) as f32);
        out.set(2, 0, dot(self.get_col(2), other.get_row(0)) as f32);
        out.set(3, 0, dot(self.get_col(3), other.get_row(0)) as f32);

        out.set(0, 1, dot(self.get_col(0), other.get_row(1)) as f32);
        out.set(1, 1, dot(self.get_col(1), other.get_row(1)) as f32);
        out.set(2, 1, dot(self.get_col(2), other.get_row(1)) as f32);
        out.set(3, 1, dot(self.get_col(3), other.get_row(1)) as f32);

        out.set(0, 2, dot(self.get_col(0), other.get_row(2)) as f32);
        out.set(1, 2, dot(self.get_col(1), other.get_row(2)) as f32);
        out.set(2, 2, dot(self.get_col(2), other.get_row(2)) as f32);
        out.set(3, 2, dot(self.get_col(3), other.get_row(2)) as f32);

        out.set(0, 3, dot(self.get_col(0), other.get_row(3)) as f32);
        out.set(1, 3, dot(self.get_col(1), other.get_row(3)) as f32);
        out.set(2, 3, dot(self.get_col(2), other.get_row(3)) as f32);
        out.set(3, 3, dot(self.get_col(3), other.get_row(3)) as f32);
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

/// Takes a rotation (the rotation around the X, Y, and Z axis), and
/// creates a normalized vector ray in the facing direction.<p>
/// the rotation values should be in radians (0..TAU)
pub fn axis_rot_to_ray(rot: Vec3f) -> Vec3f {
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
    vec3f!(x, y, z)
}

#[derive(Clone, Copy)]
pub struct HitResult {
    pub pos: Vec3i,
    pub face: Vec3i,
}
pub fn cast_ray(
    start: Vec3f,
    dir: Vec3f,
    max_dist: f32,
    collides: impl Fn(Vec3i) -> bool,
) -> Option<HitResult> {
    // -- DDA algorithm --

    // length of a line in same direction as the ray,
    // that travels 1 unit in the X, Y, Z
    let unit_step_size = vec3f!(
        (1.0 + (dir.y / dir.x) * (dir.y / dir.x) + (dir.z / dir.x) * (dir.z / dir.x)).sqrt(),
        (1.0 + (dir.x / dir.y) * (dir.x / dir.y) + (dir.z / dir.y) * (dir.z / dir.y)).sqrt(),
        (1.0 + (dir.x / dir.z) * (dir.x / dir.z) + (dir.y / dir.z) * (dir.y / dir.z)).sqrt(),
    );

    let mut map_check = start.floor();

    let (step, mut ray_len1d): (Vec3f, Vec3f) = {
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
            vec3f!(step_x, step_y, step_z),
            vec3f!(ray_len_x, ray_len_y, ray_len_z),
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
