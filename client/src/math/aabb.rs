use glam::Vec3;

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
