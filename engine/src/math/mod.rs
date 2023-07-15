pub mod aabb;
pub mod dda;

use glam::{ivec3, IVec3};

/// # Examples
/// ```rust
/// let mut field = vrt_engine::world::BitField::ZERO;
///
/// field.set(1, 1, 1);
/// assert_eq!(field.raw(), 0b00000000_00000010);
///
/// field.set(1, 1, 2);
/// assert_eq!(field.raw(), 0b00000000_00000110);
///
/// field.set(0b101, 3, 5);
/// assert_eq!(field.raw(), 0b00000000_10100110);
///
/// field.set(0b11011101, 8, 8);
/// assert_eq!(field.raw(), 0b11011101_10100110);
///
/// assert_eq!(field.get(1, 0), 0);
/// assert_eq!(field.get(1, 1), 1);
/// assert_eq!(field.get(2, 0), 0b10);
/// assert_eq!(field.get(2, 1), 0b11);
///
/// field.set(0, 8, 0);
/// assert_eq!(field.raw(), 0b11011101_00000000);
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct BitField(u32);
impl BitField {
    pub const ZERO: Self = Self(0);

    #[inline(always)]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub fn set(&mut self, data: u32, len: u32, offset: u32) {
        let mask = !(!0 << len) << offset;
        self.0 = (self.0 & !mask) | (data << offset);
    }

    #[inline(always)]
    pub fn get(self, len: u32, offset: u32) -> u32 {
        let mask = !(!0 << len) << offset;
        (self.0 & mask) >> offset
    }
}

pub fn walk_line(mut a: IVec3, b: IVec3) -> Vec<IVec3> {
    let mut result = Vec::new();
    result.push(a);

    let dist = (b - a).abs();
    let step = ivec3(
        (b.x > a.x) as i32 * 2 - 1,
        (b.y > a.y) as i32 * 2 - 1,
        (b.z > a.z) as i32 * 2 - 1,
    );

    if dist.x >= dist.y && dist.x >= dist.z {
        let mut p1 = 2 * dist.y - dist.x;
        let mut p2 = 2 * dist.z - dist.x;
        while a.x != b.x {
            a.x += step.x;
            if p1 >= 0 {
                a.y += step.y;
                p1 -= 2 * dist.x;
            }
            if p2 >= 0 {
                a.z += step.z;
                p2 -= 2 * dist.x;
            }
            p1 += 2 * dist.y;
            p2 += 2 * dist.z;
            result.push(a);
        }
    } else if dist.y >= dist.x && dist.y >= dist.z {
        let mut p1 = 2 * dist.x - dist.y;
        let mut p2 = 2 * dist.z - dist.y;
        while a.y != b.y {
            a.y += step.y;
            if p1 >= 0 {
                a.x += step.x;
                p1 -= 2 * dist.y;
            }
            if p2 >= 0 {
                a.z += step.z;
                p2 -= 2 * dist.y;
            }
            p1 += 2 * dist.x;
            p2 += 2 * dist.z;
            result.push(a);
        }
    } else {
        let mut p1 = 2 * dist.y - dist.z;
        let mut p2 = 2 * dist.x - dist.z;
        while a.z != b.z {
            a.z += step.z;
            if p1 >= 0 {
                a.y += step.y;
                p1 -= 2 * dist.z;
            }
            if p2 >= 0 {
                a.x += step.x;
                p2 -= 2 * dist.z;
            }
            p1 += 2 * dist.y;
            p2 += 2 * dist.x;
            result.push(a);
        }
    }
    result
}
