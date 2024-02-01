pub mod aabb;
pub mod dda;

use glam::{ivec3, vec3, IVec3, Vec3};

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

pub fn rand_cardinal_dir() -> IVec3 {
    [
        ivec3(-1, 0, 0),
        ivec3(1, 0, 0),
        ivec3(0, 0, -1),
        ivec3(0, 0, 1),
    ][fastrand::usize(0..4)]
}

pub fn rand_dir() -> Vec3 {
    fn rand_norm() -> f32 {
        let theta = 2.0 * 3.14159265 * fastrand::f32();
        let rho = (-2.0 * fastrand::f32().ln()).sqrt();
        return rho * theta.cos();
    }

    let x = rand_norm();
    let y = rand_norm();
    let z = rand_norm();
    vec3(x, y, z).normalize()
}

pub fn rand_hem_dir(norm: Vec3) -> Vec3 {
    let dir = rand_dir();
    dir * norm.dot(dir).signum()
}
