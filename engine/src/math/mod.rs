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

struct LineWalker {
    a: IVec3,
    b: IVec3,
    dist: IVec3,
    step: IVec3,
    p1: i32,
    p2: i32,
    mode: u8,
}
impl Iterator for LineWalker {
    type Item = IVec3;

    fn next(&mut self) -> Option<IVec3> {
        match self.mode {
            0 => {
                if self.a.x != self.b.x {
                    self.a.x += self.step.x;
                    if self.p1 >= 0 {
                        self.a.y += self.step.y;
                        self.p1 -= 2 * self.dist.x;
                    }
                    if self.p2 >= 0 {
                        self.a.z += self.step.z;
                        self.p2 -= 2 * self.dist.x;
                    }
                    self.p1 += 2 * self.dist.y;
                    self.p2 += 2 * self.dist.z;
                    return Some(self.a);
                }
                None
            }
            1 => {
                if self.a.y != self.b.y {
                    self.a.y += self.step.y;
                    if self.p1 >= 0 {
                        self.a.x += self.step.x;
                        self.p1 -= 2 * self.dist.y;
                    }
                    if self.p2 >= 0 {
                        self.a.z += self.step.z;
                        self.p2 -= 2 * self.dist.y;
                    }
                    self.p1 += 2 * self.dist.x;
                    self.p2 += 2 * self.dist.z;
                    return Some(self.a);
                }
                None
            }
            2 => {
                if self.a.z != self.b.z {
                    self.a.z += self.step.z;
                    if self.p1 >= 0 {
                        self.a.y += self.step.y;
                        self.p1 -= 2 * self.dist.z;
                    }
                    if self.p2 >= 0 {
                        self.a.x += self.step.x;
                        self.p2 -= 2 * self.dist.z;
                    }
                    self.p1 += 2 * self.dist.y;
                    self.p2 += 2 * self.dist.x;
                    return Some(self.a);
                }
                None
            }
            _ => unreachable!(),
        }
    }
}

pub fn walk_line(a: IVec3, b: IVec3) -> impl Iterator<Item = IVec3> {
    let dist = (b - a).abs();
    let step = ivec3(
        (b.x > a.x) as i32 * 2 - 1,
        (b.y > a.y) as i32 * 2 - 1,
        (b.z > a.z) as i32 * 2 - 1,
    );

    let (mode, p1, p2) = if dist.x >= dist.y && dist.x >= dist.z {
        (0, 2 * dist.y - dist.x, 2 * dist.z - dist.x)
    } else if dist.y >= dist.x && dist.y >= dist.z {
        (1, 2 * dist.x - dist.y, 2 * dist.z - dist.y)
    } else {
        (2, 2 * dist.y - dist.z, 2 * dist.x - dist.z)
    };

    let walker = LineWalker {
        a,
        b,
        dist,
        step,
        p1,
        p2,
        mode,
    };
    std::iter::once(a).chain(walker)
}
