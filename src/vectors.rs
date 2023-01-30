pub trait ToFromF64 {
    fn to_f64(self) -> f64;
    fn from_f64(v: f64) -> Self;
}
macro_rules! impl_tofromf64_cast {
	($ty:ty) => {
		impl ToFromF64 for $ty {
			#[inline(always)] fn to_f64(self) -> f64 { self as f64 }
			#[inline(always)] fn from_f64(v:f64) -> Self { v as $ty }
		}
	};
	($($ty:ty),*) => {
		$(impl_tofromf64_cast!($ty);)*
	}
}
impl_tofromf64_cast!(i8, u8, i16, u16, i32, u32, i64, u64, f32, f64);

macro_rules! impl_math_struct_op {
	($name:ident{$($field:ident),*},$op_trait:ident,$op_fn:ident,$op_tt:tt) => {
		impl<T: std::ops::$op_trait<Output = T>> std::ops::$op_trait<Self> for $name<T> {
			type Output = Self;
			fn $op_fn(self, rhs:Self) -> Self::Output {
				Self {
					$($field: self.$field $op_tt rhs.$field,)*
				}
			}
		}
		impl<T: std::ops::$op_trait<Output = T> + Copy> std::ops::$op_trait<T> for $name<T> {
			type Output = Self;
			fn $op_fn(self, rhs:T) -> Self::Output {
				Self {
					$($field: self.$field $op_tt rhs,)*
				}
			}
		}
	};
	(asn,$name:ident{$($field:ident),*},$op_trait:ident,$op_fn:ident,$op_tt:tt) => {
		impl<T: std::ops::$op_trait> std::ops::$op_trait<Self> for $name<T> {
			fn $op_fn(&mut self, rhs:Self) {
				$(self.$field $op_tt rhs.$field;)*
			}
		}
		impl<T: std::ops::$op_trait + Copy> std::ops::$op_trait<T> for $name<T> {
			fn $op_fn(&mut self, rhs:T) {
				$(self.$field $op_tt rhs;)*
			}
		}
	}
}
macro_rules! define_math_struct {
	($name:ident[$size:literal]{$($field:ident),*}) => {
		#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(C)]
		pub struct $name<T> {
			$(pub $field: T,)*
		}

		impl<T: Copy> $name<T> {
			#[inline(always)] pub const fn new($($field:T),*) -> Self {Self { $($field),* }}
			#[inline(always)] pub const fn all(v:T) -> Self {Self { $($field:v),* }}
		}
		impl<T: Copy> $name<T> {
			#[inline(always)] pub fn to_arr(self) -> [T; $size] { [$(self.$field),*] }

			#[inline(always)] pub fn map<E>(&self, f:impl Fn(T) -> E) -> $name<E> {
				$name { $($field: f(self.$field)),* }
			}
		}
		impl_math_struct_op!($name{$($field),*},Add,add,+);
		impl_math_struct_op!($name{$($field),*},Sub,sub,-);
		impl_math_struct_op!($name{$($field),*},Mul,mul,*);
		impl_math_struct_op!($name{$($field),*},Div,div,/);
        impl_math_struct_op!($name{$($field),*},Rem,rem,%);
		impl_math_struct_op!(asn,$name{$($field),*},AddAssign,add_assign,+=);
		impl_math_struct_op!(asn,$name{$($field),*},SubAssign,sub_assign,-=);
		impl_math_struct_op!(asn,$name{$($field),*},MulAssign,mul_assign,*=);
		impl_math_struct_op!(asn,$name{$($field),*},DivAssign,div_assign,/=);
        impl_math_struct_op!(asn,$name{$($field),*},RemAssign,rem_assign,%=);
	}
}

define_math_struct!(Vec2[2]{x,y});
define_math_struct!(Vec3[3]{x,y,z});
define_math_struct!(Vec4[4]{x,y,z,w});

macro_rules! impl_math_struct_consts {
	($name:ident{$($field:ident),*}; f.$ty:ty) => {
		impl $name<$ty> {
			#[inline(always)] pub fn zero() -> Self {
				Self{ $($field: 0.0),* }
			}
			#[inline(always)] pub fn one() -> Self {
				Self{ $($field: 1.0),* }
			}
		}
	};
	($name:ident{$($field:ident),*}; i.$ty:ty) => {
		impl $name<$ty> {
			#[inline(always)] pub fn zero() -> Self {
				Self{ $($field: 0),* }
			}
			#[inline(always)] pub fn one() -> Self {
				Self{ $($field: 1),* }
			}
		}
	};
}
impl_math_struct_consts!(Vec2{x,y}; f.f32);
impl_math_struct_consts!(Vec2{x,y}; f.f64);
impl_math_struct_consts!(Vec2{x,y}; i.i64);
impl_math_struct_consts!(Vec2{x,y}; i.i32);

impl_math_struct_consts!(Vec3{x,y,z}; f.f32);
impl_math_struct_consts!(Vec3{x,y,z}; f.f64);
impl_math_struct_consts!(Vec3{x,y,z}; i.i64);
impl_math_struct_consts!(Vec3{x,y,z}; i.i32);

impl_math_struct_consts!(Vec4{x,y,z,w}; f.f32);
impl_math_struct_consts!(Vec4{x,y,z,w}; f.f64);
impl_math_struct_consts!(Vec4{x,y,z,w}; i.i64);
impl_math_struct_consts!(Vec4{x,y,z,w}; i.i32);

impl<T: Copy> From<(T, T)> for Vec2<T> {
    fn from(v: (T, T)) -> Self {
        Self { x: v.0, y: v.1 }
    }
}
impl<T: Copy> From<[T; 2]> for Vec2<T> {
    fn from(v: [T; 2]) -> Self {
        Self { x: v[0], y: v[1] }
    }
}
impl<T: Copy> From<Vec2<T>> for [T; 2] {
    fn from(v: Vec2<T>) -> Self {
        [v.x, v.y]
    }
}

impl<T: Copy> From<(T, T, T)> for Vec3<T> {
    fn from(v: (T, T, T)) -> Self {
        Self {
            x: v.0,
            y: v.1,
            z: v.2,
        }
    }
}
impl<T: Copy> From<[T; 3]> for Vec3<T> {
    fn from(v: [T; 3]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
        }
    }
}
impl<T: Copy> From<Vec3<T>> for [T; 3] {
    fn from(v: Vec3<T>) -> Self {
        [v.x, v.y, v.z]
    }
}

impl<T: Copy> From<(T, T, T, T)> for Vec4<T> {
    fn from(v: (T, T, T, T)) -> Self {
        Self {
            x: v.0,
            y: v.1,
            z: v.2,
            w: v.3,
        }
    }
}
impl<T: Copy> From<[T; 4]> for Vec4<T> {
    fn from(v: [T; 4]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
            w: v[3],
        }
    }
}

impl Vec3<i32> {
    pub fn unsigned(self) -> Option<Vec3<u32>> {
        if self.x < 0 || self.y < 0 || self.z < 0 {
            return None;
        }
        Some(Vec3::new(self.x as u32, self.y as u32, self.z as u32))
    }
}

pub trait VecMath {
    /// The length of this vector, squared, assuming an origin of (0, 0).
    fn len_sq(&self) -> f64;

    /// The dot product of this and some other vector.
    fn dot(self, other: Self) -> f64;

    /// The cross product of this and some other vector.
    fn cross(self, other: Self) -> f64;

    /// Scales down this vector to have a length of 1
    fn norm(self) -> Self;

    /// The length of this vector, assuming an origin of (0, 0).
    #[inline(always)]
    fn len(&self) -> f64 {
        self.len_sq().sqrt()
    }
    #[inline(always)]
    fn abs_len_sq(&self) -> f64 {
        self.len_sq().abs()
    }
    #[inline(always)]
    fn abs_len(&self) -> f64 {
        self.abs_len_sq().sqrt()
    }
}

impl<T: ToFromF64 + Copy> VecMath for Vec2<T> {
    #[inline(always)]
    fn len_sq(&self) -> f64 {
        self.x.to_f64() * self.x.to_f64() + self.y.to_f64() * self.y.to_f64()
    }

    #[inline(always)]
    fn dot(self, other: Self) -> f64 {
        self.x.to_f64() * other.x.to_f64() + self.y.to_f64() * other.y.to_f64()
    }

    #[inline(always)]
    fn cross(self, other: Self) -> f64 {
        self.x.to_f64() * other.y.to_f64() - self.y.to_f64() * other.x.to_f64()
    }

    #[inline(always)]
    fn norm(self) -> Self {
        let len = self.len();
        Self {
            x: T::from_f64(self.x.to_f64() / len),
            y: T::from_f64(self.y.to_f64() / len),
        }
    }
}
impl<T: ToFromF64 + Copy> VecMath for Vec3<T> {
    #[inline(always)]
    fn len_sq(&self) -> f64 {
        self.x.to_f64() * self.x.to_f64()
            + self.y.to_f64() * self.y.to_f64()
            + self.z.to_f64() * self.z.to_f64()
    }

    #[inline(always)]
    fn dot(self, other: Self) -> f64 {
        self.x.to_f64() * other.x.to_f64()
            + self.y.to_f64() * other.y.to_f64()
            + self.z.to_f64() * other.z.to_f64()
    }

    #[inline(always)]
    fn cross(self, _: Self) -> f64 {
        unimplemented!()
    }

    #[inline(always)]
    fn norm(self) -> Self {
        let len = self.len();
        Self {
            x: T::from_f64(self.x.to_f64() / len),
            y: T::from_f64(self.y.to_f64() / len),
            z: T::from_f64(self.z.to_f64() / len),
        }
    }
}
impl<T: Copy> Vec3<T> {
    #[inline(always)]
    pub fn drop_x(self) -> Vec2<T> {
        Vec2::new(self.y, self.z)
    }
    #[inline(always)]
    pub fn drop_y(self) -> Vec2<T> {
        Vec2::new(self.x, self.z)
    }
    #[inline(always)]
    pub fn drop_z(self) -> Vec2<T> {
        Vec2::new(self.y, self.z)
    }
}
impl<T: Copy + ToFromF64> Vec3<T> {
    #[inline(always)]
    pub fn keep_x(self) -> Vec3<T> {
        Vec3::new(self.x, T::from_f64(0.0), T::from_f64(0.0))
    }
    #[inline(always)]
    pub fn keep_y(self) -> Vec3<T> {
        Vec3::new(T::from_f64(0.0), self.y, T::from_f64(0.0))
    }
    #[inline(always)]
    pub fn keep_z(self) -> Vec3<T> {
        Vec3::new(T::from_f64(0.0), T::from_f64(0.0), self.z)
    }
    #[inline(always)]
    pub fn keep_xy(self) -> Vec3<T> {
        Vec3::new(self.x, self.y, T::from_f64(0.0))
    }
    #[inline(always)]
    pub fn keep_xz(self) -> Vec3<T> {
        Vec3::new(self.x, T::from_f64(0.0), self.z)
    }
    #[inline(always)]
    pub fn keep_yz(self) -> Vec3<T> {
        Vec3::new(T::from_f64(0.0), self.y, self.z)
    }
}
impl<T: ToFromF64 + Copy> VecMath for Vec4<T> {
    #[inline(always)]
    fn len_sq(&self) -> f64 {
        self.x.to_f64() * self.x.to_f64()
            + self.y.to_f64() * self.y.to_f64()
            + self.z.to_f64() * self.z.to_f64()
            + self.w.to_f64() * self.w.to_f64()
    }

    #[inline(always)]
    fn dot(self, other: Self) -> f64 {
        self.x.to_f64() * other.x.to_f64()
            + self.y.to_f64() * other.y.to_f64()
            + self.z.to_f64() * other.z.to_f64()
            + self.w.to_f64() * other.w.to_f64()
    }

    /// Not implemented for Vec4
    #[inline(always)]
    fn cross(self, _other: Self) -> f64 {
        unimplemented!()
    }

    #[inline(always)]
    fn norm(self) -> Self {
        let len = self.len();
        Self {
            x: T::from_f64(self.x.to_f64() / len),
            y: T::from_f64(self.y.to_f64() / len),
            z: T::from_f64(self.z.to_f64() / len),
            w: T::from_f64(self.w.to_f64() / len),
        }
    }
}
