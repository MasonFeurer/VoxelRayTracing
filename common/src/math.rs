use glam::{ivec3, vec3, IVec3, Vec3};

const EPSILON: f32 = 0.00001;

#[derive(Clone, Copy)]
pub struct Aabb {
    pub from: Vec3,
    pub to: Vec3,
}
impl Aabb {
    pub const UNIT: Self = Self::new(Vec3::ZERO, Vec3::ONE);

    #[inline(always)]
    pub const fn new(from: Vec3, to: Vec3) -> Self {
        Self { from, to }
    }

    pub fn expand(&self, a: Vec3) -> Self {
        let mut from = self.from;
        let mut to = self.to;

        if a.x < 0.0 {
            from.x += a.x
        }
        if a.x > 0.0 {
            to.x += a.x
        }

        if a.y < 0.0 {
            from.y += a.y
        }
        if a.y > 0.0 {
            to.y += a.y
        }

        if a.z < 0.0 {
            from.z += a.z
        }
        if a.z > 0.0 {
            to.z += a.z
        }

        Self::new(from, to)
    }

    pub fn grow(&self, a: Vec3) -> Self {
        Self::new(self.from - a, self.to + a)
    }

    pub fn clip_x_collide(&self, c: &Self, mut a: f32) -> f32 {
        if c.to.y <= self.from.y || c.from.y >= self.to.y {
            return a;
        }
        if c.to.z <= self.from.z || c.from.z >= self.to.z {
            return a;
        }

        if a > 0.0 && c.to.x <= self.from.x {
            let max = self.from.x - c.to.x - EPSILON;
            if max < a {
                a = max
            }
        }
        if a < 0.0 && c.from.x >= self.to.x {
            let max = self.to.x - c.from.x + EPSILON;
            if max > a {
                a = max
            }
        }
        a
    }
    pub fn clip_y_collide(&self, c: &Self, mut a: f32) -> f32 {
        if c.to.x <= self.from.x || c.from.x >= self.to.x {
            return a;
        }
        if c.to.z <= self.from.z || c.from.z >= self.to.z {
            return a;
        }

        if a > 0.0 && c.to.y <= self.from.y {
            let max = self.from.y - c.to.y - EPSILON;
            if max < a {
                a = max
            }
        }
        if a < 0.0 && c.from.y >= self.to.y {
            let max = self.to.y - c.from.y + EPSILON;
            if max > a {
                a = max
            }
        }
        a
    }
    pub fn clip_z_collide(&self, c: &Self, mut a: f32) -> f32 {
        if c.to.x <= self.from.x || c.from.x >= self.to.x {
            return a;
        }
        if c.to.y <= self.from.y || c.from.y >= self.to.y {
            return a;
        }

        if a > 0.0 && c.to.z <= self.from.z {
            let max = self.from.z - c.to.z - EPSILON;
            if max < a {
                a = max
            }
        }
        if a < 0.0 && c.from.z >= self.to.z {
            let max = self.to.z - c.from.z + EPSILON;
            if max > a {
                a = max
            }
        }
        a
    }

    pub fn intersects(&self, c: &Self) -> bool {
        (c.to.x > self.from.x && c.from.x < self.to.x)
            && (c.to.y > self.from.y && c.from.y < self.to.y)
            && (c.to.z > self.from.z && c.from.z < self.to.z)
    }

    pub fn translate(&mut self, a: Vec3) {
        self.from += a;
        self.to += a;
    }
}

/// Takes a rotation (the rotation around the X, Y, and Z axis), and
/// creates a normalized vector ray in the facing direction.
/// the rotation values should be in radians (0..TAU)
pub fn axis_rot_to_ray(rot: Vec3) -> Vec3 {
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
    vec3(x, y, z)
}

#[derive(Clone, Copy)]
pub struct HitResult {
    pub pos: IVec3,
    pub face: IVec3,
}
pub fn cast_ray(
    start: Vec3,
    dir: Vec3,
    max_dist: f32,
    collides: impl Fn(IVec3) -> bool,
) -> Option<HitResult> {
    // -- DDA algorithm --

    // length of a line in same direction as the ray,
    // that travels 1 unit in the X, Y, Z
    let unit_step_size = vec3(
        (1.0 + (dir.y / dir.x) * (dir.y / dir.x) + (dir.z / dir.x) * (dir.z / dir.x)).sqrt(),
        (1.0 + (dir.x / dir.y) * (dir.x / dir.y) + (dir.z / dir.y) * (dir.z / dir.y)).sqrt(),
        (1.0 + (dir.x / dir.z) * (dir.x / dir.z) + (dir.y / dir.z) * (dir.y / dir.z)).sqrt(),
    );

    let mut map_check = start.floor().as_ivec3();

    let (step, mut ray_len1d): (Vec3, Vec3) = {
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
            vec3(step_x, step_y, step_z),
            vec3(ray_len_x, ray_len_y, ray_len_z),
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

struct LineWalker {
    a: IVec3,
    b: IVec3,
    dist: IVec3,
    step: IVec3,
    p1: i32,
    p2: i32,
    // TODO mode can be a struct generic type (it is only set at struct construction time)
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
        let theta: f32 = 2.0 * 3.14159265 * fastrand::f32();
        let rho = (-2.0f32 * fastrand::f32().ln()).sqrt();
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
