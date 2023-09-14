//!
//! 2014 OpenSimplex Noise in Java.
//! by Kurt Spencer
//!
//! Ported to Rust by Mason Feurer (Excluding 4D) (Added `NoiseMap` and `MultiNoiseMap`).
//! I have no idea how this thing works, just translated the control flow.
//!
use glam::Vec2;

#[derive(Clone)]
pub struct NoiseMap {
    // OpenSimplexNoise is a pretty big struct, better to not store it on the stack
    noise: Box<OpenSimplexNoise>,
    scale: f64,
    freq: f64,
}
impl NoiseMap {
    pub fn new(seed: i64, freq: f64, scale: f64) -> Self {
        Self {
            noise: Box::new(OpenSimplexNoise::new(seed)),
            freq,
            scale,
        }
    }
    pub fn get(&self, pos: Vec2) -> f32 {
        let val = self
            .noise
            .eval2d(pos.x as f64 * self.freq, pos.y as f64 * self.freq);
        (((val + 1.0) * 0.5) * self.scale) as f32
    }
}

const STRETCH_CONSTANT_2D: f64 = -0.211324865405187; // (1/(2+1).sqrt()-1)/2;
const SQUISH_CONSTANT_2D: f64 = 0.366025403784439; // ((2+1).sqrt()-1)/2;
const STRETCH_CONSTANT_3D: f64 = -1.0 / 6.0; // (1/(3+1).sqrt()-1)/3;
const SQUISH_CONSTANT_3D: f64 = 1.0 / 3.0; // ((3+1).sqrt()-1)/3;

const PSIZE: usize = 2048;
const PMASK: usize = 2047;

#[derive(Clone)]
pub struct OpenSimplexNoise {
    perm: [usize; PSIZE],
    perm_grad2: [Grad2; PSIZE],
    perm_grad3: [Grad3; PSIZE],
}
impl OpenSimplexNoise {
    pub fn new(mut seed: i64) -> Self {
        let mut perm = [0; PSIZE];
        let mut perm_grad2 = [Grad2::ZERO; PSIZE];
        let mut perm_grad3 = [Grad3::ZERO; PSIZE];
        let mut source = [0; PSIZE];
        for i in 0..PSIZE {
            source[i] = i;
        }
        for i in (0..PSIZE).rev() {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let r = ((seed + 31) % (i as i64 + 1)) as isize;
            let r = if r < 0 { r + i as isize + 1 } else { r } as usize;
            perm[i] = source[r];
            perm_grad2[i] = unsafe { GRADIENTS_2D[perm[i]] };
            perm_grad3[i] = unsafe { GRADIENTS_3D[perm[i]] };
            source[r] = source[i];
        }
        Self {
            perm,
            perm_grad2,
            perm_grad3,
        }
    }

    /// 2D OpenSimplex Noise
    pub fn eval2d(&self, x: f64, y: f64) -> f64 {
        // Place input coordinates onto grid.
        let stretch_offset = (x + y) * STRETCH_CONSTANT_2D;
        let xs = x + stretch_offset;
        let ys = y + stretch_offset;

        // Floor to get grid coordinates of rhombus (stretched square) super-cell origin.
        let mut xsb: i32 = xs.floor() as i32;
        let mut ysb: i32 = ys.floor() as i32;

        // Compute grid coordinates relative to rhombus origin.
        let xins: f64 = xs - xsb as f64;
        let yins: f64 = ys - ysb as f64;

        // Sum those together to get a value that determines which region we're in.
        let in_sum: f64 = xins + yins;

        // Positions relative to origin point.
        let squish_offset_ins: f64 = in_sum * SQUISH_CONSTANT_2D;
        let mut dx0: f64 = xins + squish_offset_ins;
        let mut dy0: f64 = yins + squish_offset_ins;

        // We'll be defining these inside the next block and using them afterwards.
        let dx_ext: f64;
        let dy_ext: f64;
        let xsv_ext: i32;
        let ysv_ext: i32;

        let mut value: f64 = 0.0;

        // Contribution (1,0)
        let dx1: f64 = dx0 - 1.0 - SQUISH_CONSTANT_2D;
        let dy1: f64 = dy0 - 0.0 - SQUISH_CONSTANT_2D;
        let mut attn1: f64 = 2.0 - dx1 * dx1 - dy1 * dy1;
        if attn1 > 0.0 {
            attn1 *= attn1;
            value += attn1 * attn1 * self.extrapolate2d(xsb + 1, ysb + 0, dx1, dy1);
        }

        // Contribution (0,1)
        let dx2: f64 = dx0 - 0.0 - SQUISH_CONSTANT_2D;
        let dy2: f64 = dy0 - 1.0 - SQUISH_CONSTANT_2D;
        let mut attn2: f64 = 2.0 - dx2 * dx2 - dy2 * dy2;
        if attn2 > 0.0 {
            attn2 *= attn2;
            value += attn2 * attn2 * self.extrapolate2d(xsb + 0, ysb + 1, dx2, dy2);
        }

        if in_sum <= 1.0 {
            // We're inside the triangle (2-Simplex) at (0,0)
            let zins: f64 = 1.0 - in_sum;
            if zins > xins || zins > yins {
                // (0,0) is one of the closest two triangular vertices
                if xins > yins {
                    xsv_ext = xsb + 1;
                    ysv_ext = ysb - 1;
                    dx_ext = dx0 - 1.0;
                    dy_ext = dy0 + 1.0;
                } else {
                    xsv_ext = xsb - 1;
                    ysv_ext = ysb + 1;
                    dx_ext = dx0 + 1.0;
                    dy_ext = dy0 - 1.0;
                }
            } else {
                // (1,0) and (0,1) are the closest two vertices.
                xsv_ext = xsb + 1;
                ysv_ext = ysb + 1;
                dx_ext = dx0 - 1.0 - 2.0 * SQUISH_CONSTANT_2D;
                dy_ext = dy0 - 1.0 - 2.0 * SQUISH_CONSTANT_2D;
            }
        } else {
            // We're inside the triangle (2-Simplex) at (1,1)
            let zins: f64 = 2.0 - in_sum;
            if zins < xins || zins < yins {
                // (0,0) is one of the closest two triangular vertices
                if xins > yins {
                    xsv_ext = xsb + 2;
                    ysv_ext = ysb + 0;
                    dx_ext = dx0 - 2.0 - 2.0 * SQUISH_CONSTANT_2D;
                    dy_ext = dy0 + 0.0 - 2.0 * SQUISH_CONSTANT_2D;
                } else {
                    xsv_ext = xsb + 0;
                    ysv_ext = ysb + 2;
                    dx_ext = dx0 + 0.0 - 2.0 * SQUISH_CONSTANT_2D;
                    dy_ext = dy0 - 2.0 - 2.0 * SQUISH_CONSTANT_2D;
                }
            } else {
                // (1,0) and (0,1) are the closest two vertices.
                dx_ext = dx0;
                dy_ext = dy0;
                xsv_ext = xsb;
                ysv_ext = ysb;
            }
            xsb += 1;
            ysb += 1;
            dx0 = dx0 - 1.0 - 2.0 * SQUISH_CONSTANT_2D;
            dy0 = dy0 - 1.0 - 2.0 * SQUISH_CONSTANT_2D;
        }

        // Contribution (0,0) or (1,1)
        let mut attn0: f64 = 2.0 - dx0 * dx0 - dy0 * dy0;
        if attn0 > 0.0 {
            attn0 *= attn0;
            value += attn0 * attn0 * self.extrapolate2d(xsb, ysb, dx0, dy0);
        }

        // Extra Vertex
        let mut attn_ext: f64 = 2.0 - dx_ext * dx_ext - dy_ext * dy_ext;
        if attn_ext > 0.0 {
            attn_ext *= attn_ext;
            value += attn_ext * attn_ext * self.extrapolate2d(xsv_ext, ysv_ext, dx_ext, dy_ext);
        }
        value
    }

    /// 3D OpenSimplex Noise
    pub fn eval3d(&self, x: f64, y: f64, z: f64) -> f64 {
        // Place input coordinates on simplectic honeycomb.
        let stretch_offset: f64 = (x + y + z) * STRETCH_CONSTANT_3D;
        let xs: f64 = x + stretch_offset;
        let ys: f64 = y + stretch_offset;
        let zs: f64 = z + stretch_offset;
        self.eval3_base(xs, ys, zs)
    }

    /// Not as good as in SuperSimplex/OpenSimplex2S, since there are more visible differences between different slices.
    /// The Z coordinate should always be the "different" coordinate in your use case.
    pub fn eval3_xy_before_z(&self, x: f64, y: f64, z: f64) -> f64 {
        // Combine rotation with skew transform.
        let xy: f64 = x + y;
        let s2: f64 = xy * 0.211324865405187;
        let zz: f64 = z * 0.288675134594813;
        let xs: f64 = s2 - x + zz;
        let ys: f64 = s2 - y + zz;
        let zs: f64 = xy * 0.577350269189626 + zz;

        self.eval3_base(xs, ys, zs)
    }

    // Similar to the above, except the Y coordinate should always be the "different" coordinate in your use case.
    pub fn eval3_xz_before_y(&self, x: f64, y: f64, z: f64) -> f64 {
        // Combine rotation with skew transform.
        let xz: f64 = x + z;
        let s2: f64 = xz * 0.211324865405187;
        let yy: f64 = y * 0.288675134594813;
        let xs: f64 = s2 - x + yy;
        let zs: f64 = s2 - z + yy;
        let ys: f64 = xz * 0.577350269189626 + yy;
        self.eval3_base(xs, ys, zs)
    }

    // 3D OpenSimplex Noise (base which takes skewed coordinates directly).
    fn eval3_base(&self, xs: f64, ys: f64, zs: f64) -> f64 {
        // Floor to get simplectic honeycomb coordinates of rhombohedron (stretched cube) super-cell origin.
        let xsb: i32 = xs.floor() as i32;
        let ysb: i32 = ys.floor() as i32;
        let zsb: i32 = zs.floor() as i32;

        // Compute simplectic honeycomb coordinates relative to rhombohedral origin.
        let xins: f64 = xs - xsb as f64;
        let yins: f64 = ys - ysb as f64;
        let zins: f64 = zs - zsb as f64;

        // Sum those together to get a value that determines which region we're in.
        let in_sum: f64 = xins + yins + zins;

        // Positions relative to origin point.
        let squish_offset_ins: f64 = in_sum * SQUISH_CONSTANT_3D;
        let mut dx0: f64 = xins + squish_offset_ins;
        let mut dy0: f64 = yins + squish_offset_ins;
        let mut dz0: f64 = zins + squish_offset_ins;

        // We'll be defining these inside the next block and using them afterwards.
        let dx_ext0: f64;
        let mut dy_ext0: f64;
        let dz_ext0: f64;
        let mut dx_ext1: f64;
        let mut dy_ext1: f64;
        let mut dz_ext1: f64;
        let xsv_ext0: i32;
        let mut ysv_ext0: i32;
        let zsv_ext0: i32;
        let mut xsv_ext1: i32;
        let mut ysv_ext1: i32;
        let mut zsv_ext1: i32;

        let mut value: f64 = 0.0;
        if in_sum <= 1.0 {
            // We're inside the tetrahedron (3-Simplex) at (0,0,0)

            // Determine which two of (0,0,1), (0,1,0), (1,0,0) are closest.
            let mut a_point: u8 = 0x01;
            let mut a_score: f64 = xins;
            let mut b_point: u8 = 0x02;
            let mut b_score: f64 = yins;
            if a_score >= b_score && zins > b_score {
                b_score = zins;
                b_point = 0x04;
            } else if a_score < b_score && zins > a_score {
                a_score = zins;
                a_point = 0x04;
            }

            // Now we determine the two lattice points not part of the tetrahedron that may contribute.
            // This depends on the closest two tetrahedral vertices, including (0,0,0)
            let wins: f64 = 1.0 - in_sum;
            if wins > a_score || wins > b_score {
                // (0,0,0) is one of the closest two tetrahedral vertices.
                let c: u8 = if b_score > a_score { b_point } else { a_point }; // Our other closest vertex is the closest out of a and b.

                if (c & 0x01) == 0 {
                    xsv_ext0 = xsb - 1;
                    xsv_ext1 = xsb;
                    dx_ext0 = dx0 + 1.0;
                    dx_ext1 = dx0;
                } else {
                    xsv_ext1 = xsb + 1;
                    xsv_ext0 = xsv_ext1;
                    dx_ext1 = dx0 - 1.0;
                    dx_ext0 = dx_ext1;
                }

                if (c & 0x02) == 0 {
                    ysv_ext1 = ysb;
                    ysv_ext0 = ysv_ext1;
                    dy_ext1 = dy0;
                    dy_ext0 = dy_ext1;
                    if (c & 0x01) == 0 {
                        ysv_ext1 -= 1;
                        dy_ext1 += 1.0;
                    } else {
                        ysv_ext0 -= 1;
                        dy_ext0 += 1.0;
                    }
                } else {
                    ysv_ext1 = ysb + 1;
                    ysv_ext0 = ysv_ext1;
                    dy_ext1 = dy0 - 1.0;
                    dy_ext0 = dy_ext1;
                }

                if (c & 0x04) == 0 {
                    zsv_ext0 = zsb;
                    zsv_ext1 = zsb - 1;
                    dz_ext0 = dz0;
                    dz_ext1 = dz0 + 1.0;
                } else {
                    zsv_ext1 = zsb + 1;
                    zsv_ext0 = zsv_ext1;
                    dz_ext1 = dz0 - 1.0;
                    dz_ext0 = dz_ext1;
                }
            } else {
                // (0,0,0) is not one of the closest two tetrahedral vertices.
                let c: u8 = a_point | b_point; // Our two extra vertices are determined by the closest two.

                if (c & 0x01) == 0 {
                    xsv_ext0 = xsb;
                    xsv_ext1 = xsb - 1;
                    dx_ext0 = dx0 - 2.0 * SQUISH_CONSTANT_3D;
                    dx_ext1 = dx0 + 1.0 - SQUISH_CONSTANT_3D;
                } else {
                    xsv_ext1 = xsb + 1;
                    xsv_ext0 = xsv_ext1;
                    dx_ext0 = dx0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
                    dx_ext1 = dx0 - 1.0 - SQUISH_CONSTANT_3D;
                }

                if (c & 0x02) == 0 {
                    ysv_ext0 = ysb;
                    ysv_ext1 = ysb - 1;
                    dy_ext0 = dy0 - 2.0 * SQUISH_CONSTANT_3D;
                    dy_ext1 = dy0 + 1.0 - SQUISH_CONSTANT_3D;
                } else {
                    ysv_ext1 = ysb + 1;
                    ysv_ext0 = ysv_ext1;
                    dy_ext0 = dy0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
                    dy_ext1 = dy0 - 1.0 - SQUISH_CONSTANT_3D;
                }

                if (c & 0x04) == 0 {
                    zsv_ext0 = zsb;
                    zsv_ext1 = zsb - 1;
                    dz_ext0 = dz0 - 2.0 * SQUISH_CONSTANT_3D;
                    dz_ext1 = dz0 + 1.0 - SQUISH_CONSTANT_3D;
                } else {
                    zsv_ext1 = zsb + 1;
                    zsv_ext0 = zsv_ext1;
                    dz_ext0 = dz0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
                    dz_ext1 = dz0 - 1.0 - SQUISH_CONSTANT_3D;
                }
            }

            // Contribution (0,0,0)
            let mut attn0: f64 = 2.0 - dx0 * dx0 - dy0 * dy0 - dz0 * dz0;
            if attn0 > 0.0 {
                attn0 *= attn0;
                value +=
                    attn0 * attn0 * self.extrapolate3d(xsb + 0, ysb + 0, zsb + 0, dx0, dy0, dz0);
            }

            // Contribution (1,0,0)
            let dx1: f64 = dx0 - 1.0 - SQUISH_CONSTANT_3D;
            let dy1: f64 = dy0 - 0.0 - SQUISH_CONSTANT_3D;
            let dz1: f64 = dz0 - 0.0 - SQUISH_CONSTANT_3D;
            let mut attn1: f64 = 2.0 - dx1 * dx1 - dy1 * dy1 - dz1 * dz1;
            if attn1 > 0.0 {
                attn1 *= attn1;
                value +=
                    attn1 * attn1 * self.extrapolate3d(xsb + 1, ysb + 0, zsb + 0, dx1, dy1, dz1);
            }

            // Contribution (0,1,0)
            let dx2: f64 = dx0 - 0.0 - SQUISH_CONSTANT_3D;
            let dy2: f64 = dy0 - 1.0 - SQUISH_CONSTANT_3D;
            let dz2: f64 = dz1;
            let mut attn2: f64 = 2.0 - dx2 * dx2 - dy2 * dy2 - dz2 * dz2;
            if attn2 > 0.0 {
                attn2 *= attn2;
                value +=
                    attn2 * attn2 * self.extrapolate3d(xsb + 0, ysb + 1, zsb + 0, dx2, dy2, dz2);
            }

            // Contribution (0,0,1)
            let dx3: f64 = dx2;
            let dy3: f64 = dy1;
            let dz3: f64 = dz0 - 1.0 - SQUISH_CONSTANT_3D;
            let mut attn3: f64 = 2.0 - dx3 * dx3 - dy3 * dy3 - dz3 * dz3;
            if attn3 > 0.0 {
                attn3 *= attn3;
                value +=
                    attn3 * attn3 * self.extrapolate3d(xsb + 0, ysb + 0, zsb + 1, dx3, dy3, dz3);
            }
        } else if in_sum >= 2.0 {
            // We're inside the tetrahedron (3-Simplex) at (1,1,1)

            // Determine which two tetrahedral vertices are the closest, out of (1,1,0), (1,0,1), (0,1,1) but not (1,1,1).
            let mut a_point: u8 = 0x06;
            let mut a_score: f64 = xins;
            let mut b_point: u8 = 0x05;
            let mut b_score: f64 = yins;
            if a_score <= b_score && zins < b_score {
                b_score = zins;
                b_point = 0x03;
            } else if a_score > b_score && zins < a_score {
                a_score = zins;
                a_point = 0x03;
            }

            // Now we determine the two lattice points not part of the tetrahedron that may contribute.
            // This depends on the closest two tetrahedral vertices, including (1,1,1)
            let wins: f64 = 3.0 - in_sum;
            if wins < a_score || wins < b_score {
                // (1,1,1) is one of the closest two tetrahedral vertices.
                let c: u8 = if b_score < a_score { b_point } else { a_point }; // Our other closest vertex is the closest out of a and b.

                if (c & 0x01) != 0 {
                    xsv_ext0 = xsb + 2;
                    xsv_ext1 = xsb + 1;
                    dx_ext0 = dx0 - 2.0 - 3.0 * SQUISH_CONSTANT_3D;
                    dx_ext1 = dx0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
                } else {
                    xsv_ext1 = xsb;
                    xsv_ext0 = xsv_ext1;
                    dx_ext1 = dx0 - 3.0 * SQUISH_CONSTANT_3D;
                    dx_ext0 = dx_ext1;
                }

                if (c & 0x02) != 0 {
                    ysv_ext1 = ysb + 1;
                    ysv_ext0 = ysv_ext1;
                    dy_ext1 = dy0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
                    dy_ext0 = dy_ext1;
                    if (c & 0x01) != 0 {
                        ysv_ext1 += 1;
                        dy_ext1 -= 1.0;
                    } else {
                        ysv_ext0 += 1;
                        dy_ext0 -= 1.0;
                    }
                } else {
                    ysv_ext1 = ysb;
                    ysv_ext0 = ysv_ext1;
                    dy_ext1 = dy0 - 3.0 * SQUISH_CONSTANT_3D;
                    dy_ext0 = dy_ext1;
                }

                if (c & 0x04) != 0 {
                    zsv_ext0 = zsb + 1;
                    zsv_ext1 = zsb + 2;
                    dz_ext0 = dz0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
                    dz_ext1 = dz0 - 2.0 - 3.0 * SQUISH_CONSTANT_3D;
                } else {
                    zsv_ext1 = zsb;
                    zsv_ext0 = zsv_ext1;
                    dz_ext1 = dz0 - 3.0 * SQUISH_CONSTANT_3D;
                    dz_ext0 = dz_ext1;
                }
            } else {
                // (1,1,1) is not one of the closest two tetrahedral vertices.
                let c: u8 = a_point & b_point; // Our two extra vertices are determined by the closest two.

                if (c & 0x01) != 0 {
                    xsv_ext0 = xsb + 1;
                    xsv_ext1 = xsb + 2;
                    dx_ext0 = dx0 - 1.0 - SQUISH_CONSTANT_3D;
                    dx_ext1 = dx0 - 2.0 - 2.0 * SQUISH_CONSTANT_3D;
                } else {
                    xsv_ext1 = xsb;
                    xsv_ext0 = xsv_ext1;
                    dx_ext0 = dx0 - SQUISH_CONSTANT_3D;
                    dx_ext1 = dx0 - 2.0 * SQUISH_CONSTANT_3D;
                }

                if (c & 0x02) != 0 {
                    ysv_ext0 = ysb + 1;
                    ysv_ext1 = ysb + 2;
                    dy_ext0 = dy0 - 1.0 - SQUISH_CONSTANT_3D;
                    dy_ext1 = dy0 - 2.0 - 2.0 * SQUISH_CONSTANT_3D;
                } else {
                    ysv_ext1 = ysb;
                    ysv_ext0 = ysv_ext1;
                    dy_ext0 = dy0 - SQUISH_CONSTANT_3D;
                    dy_ext1 = dy0 - 2.0 * SQUISH_CONSTANT_3D;
                }

                if (c & 0x04) != 0 {
                    zsv_ext0 = zsb + 1;
                    zsv_ext1 = zsb + 2;
                    dz_ext0 = dz0 - 1.0 - SQUISH_CONSTANT_3D;
                    dz_ext1 = dz0 - 2.0 - 2.0 * SQUISH_CONSTANT_3D;
                } else {
                    zsv_ext1 = zsb;
                    zsv_ext0 = zsv_ext1;
                    dz_ext0 = dz0 - SQUISH_CONSTANT_3D;
                    dz_ext1 = dz0 - 2.0 * SQUISH_CONSTANT_3D;
                }
            }

            // Contribution (1,1,0)
            let dx3: f64 = dx0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
            let dy3: f64 = dy0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
            let dz3: f64 = dz0 - 0.0 - 2.0 * SQUISH_CONSTANT_3D;
            let mut attn3: f64 = 2.0 - dx3 * dx3 - dy3 * dy3 - dz3 * dz3;
            if attn3 > 0.0 {
                attn3 *= attn3;
                value +=
                    attn3 * attn3 * self.extrapolate3d(xsb + 1, ysb + 1, zsb + 0, dx3, dy3, dz3);
            }

            // Contribution (1,0,1)
            let dx2: f64 = dx3;
            let dy2: f64 = dy0 - 0.0 - 2.0 * SQUISH_CONSTANT_3D;
            let dz2: f64 = dz0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
            let mut attn2: f64 = 2.0 - dx2 * dx2 - dy2 * dy2 - dz2 * dz2;
            if attn2 > 0.0 {
                attn2 *= attn2;
                value +=
                    attn2 * attn2 * self.extrapolate3d(xsb + 1, ysb + 0, zsb + 1, dx2, dy2, dz2);
            }

            // Contribution (0,1,1)
            let dx1: f64 = dx0 - 0.0 - 2.0 * SQUISH_CONSTANT_3D;
            let dy1: f64 = dy3;
            let dz1: f64 = dz2;
            let mut attn1: f64 = 2.0 - dx1 * dx1 - dy1 * dy1 - dz1 * dz1;
            if attn1 > 0.0 {
                attn1 *= attn1;
                value +=
                    attn1 * attn1 * self.extrapolate3d(xsb + 0, ysb + 1, zsb + 1, dx1, dy1, dz1);
            }

            // Contribution (1,1,1)
            dx0 = dx0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
            dy0 = dy0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
            dz0 = dz0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
            let mut attn0: f64 = 2.0 - dx0 * dx0 - dy0 * dy0 - dz0 * dz0;
            if attn0 > 0.0 {
                attn0 *= attn0;
                value +=
                    attn0 * attn0 * self.extrapolate3d(xsb + 1, ysb + 1, zsb + 1, dx0, dy0, dz0);
            }
        } else {
            // We're inside the octahedron (Rectified 3-Simplex) in between.
            let a_score: f64;
            let mut a_point: u8;
            let mut a_is_further_side: bool;
            let b_score: f64;
            let mut b_point: u8;
            let mut b_is_further_side: bool;

            // Decide between point (0,0,1) and (1,1,0) as closest
            let p1: f64 = xins + yins;
            if p1 > 1.0 {
                a_score = p1 - 1.0;
                a_point = 0x03;
                a_is_further_side = true;
            } else {
                a_score = 1.0 - p1;
                a_point = 0x04;
                a_is_further_side = false;
            }

            // Decide between point (0,1,0) and (1,0,1) as closest
            let p2: f64 = xins + zins;
            if p2 > 1.0 {
                b_score = p2 - 1.0;
                b_point = 0x05;
                b_is_further_side = true;
            } else {
                b_score = 1.0 - p2;
                b_point = 0x02;
                b_is_further_side = false;
            }

            // The closest out of the two (1,0,0) and (0,1,1) will replace the furthest out of the two decided above, if closer.
            let p3: f64 = yins + zins;
            if p3 > 1.0 {
                let score: f64 = p3 - 1.0;
                if a_score <= b_score && a_score < score {
                    a_point = 0x06;
                    a_is_further_side = true;
                } else if a_score > b_score && b_score < score {
                    b_point = 0x06;
                    b_is_further_side = true;
                }
            } else {
                let score: f64 = 1.0 - p3;
                if a_score <= b_score && a_score < score {
                    a_point = 0x01;
                    a_is_further_side = false;
                } else if a_score > b_score && b_score < score {
                    b_point = 0x01;
                    b_is_further_side = false;
                }
            }

            // Where each of the two closest points are determines how the extra two vertices are calculated.
            if a_is_further_side == b_is_further_side {
                if a_is_further_side {
                    // Both closest points on (1,1,1) side

                    // One of the two extra points is (1,1,1)
                    dx_ext0 = dx0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
                    dy_ext0 = dy0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
                    dz_ext0 = dz0 - 1.0 - 3.0 * SQUISH_CONSTANT_3D;
                    xsv_ext0 = xsb + 1;
                    ysv_ext0 = ysb + 1;
                    zsv_ext0 = zsb + 1;

                    // Other extra point is based on the shared axis.
                    let c: u8 = a_point & b_point;
                    if (c & 0x01) != 0 {
                        dx_ext1 = dx0 - 2.0 - 2.0 * SQUISH_CONSTANT_3D;
                        dy_ext1 = dy0 - 2.0 * SQUISH_CONSTANT_3D;
                        dz_ext1 = dz0 - 2.0 * SQUISH_CONSTANT_3D;
                        xsv_ext1 = xsb + 2;
                        ysv_ext1 = ysb;
                        zsv_ext1 = zsb;
                    } else if (c & 0x02) != 0 {
                        dx_ext1 = dx0 - 2.0 * SQUISH_CONSTANT_3D;
                        dy_ext1 = dy0 - 2.0 - 2.0 * SQUISH_CONSTANT_3D;
                        dz_ext1 = dz0 - 2.0 * SQUISH_CONSTANT_3D;
                        xsv_ext1 = xsb;
                        ysv_ext1 = ysb + 2;
                        zsv_ext1 = zsb;
                    } else {
                        dx_ext1 = dx0 - 2.0 * SQUISH_CONSTANT_3D;
                        dy_ext1 = dy0 - 2.0 * SQUISH_CONSTANT_3D;
                        dz_ext1 = dz0 - 2.0 - 2.0 * SQUISH_CONSTANT_3D;
                        xsv_ext1 = xsb;
                        ysv_ext1 = ysb;
                        zsv_ext1 = zsb + 2;
                    }
                } else {
                    // Both closest points on (0,0,0) side

                    // One of the two extra points is (0,0,0)
                    dx_ext0 = dx0;
                    dy_ext0 = dy0;
                    dz_ext0 = dz0;
                    xsv_ext0 = xsb;
                    ysv_ext0 = ysb;
                    zsv_ext0 = zsb;

                    // Other extra point is based on the omitted axis.
                    let c: u8 = a_point | b_point;
                    if (c & 0x01) == 0 {
                        dx_ext1 = dx0 + 1.0 - SQUISH_CONSTANT_3D;
                        dy_ext1 = dy0 - 1.0 - SQUISH_CONSTANT_3D;
                        dz_ext1 = dz0 - 1.0 - SQUISH_CONSTANT_3D;
                        xsv_ext1 = xsb - 1;
                        ysv_ext1 = ysb + 1;
                        zsv_ext1 = zsb + 1;
                    } else if (c & 0x02) == 0 {
                        dx_ext1 = dx0 - 1.0 - SQUISH_CONSTANT_3D;
                        dy_ext1 = dy0 + 1.0 - SQUISH_CONSTANT_3D;
                        dz_ext1 = dz0 - 1.0 - SQUISH_CONSTANT_3D;
                        xsv_ext1 = xsb + 1;
                        ysv_ext1 = ysb - 1;
                        zsv_ext1 = zsb + 1;
                    } else {
                        dx_ext1 = dx0 - 1.0 - SQUISH_CONSTANT_3D;
                        dy_ext1 = dy0 - 1.0 - SQUISH_CONSTANT_3D;
                        dz_ext1 = dz0 + 1.0 - SQUISH_CONSTANT_3D;
                        xsv_ext1 = xsb + 1;
                        ysv_ext1 = ysb + 1;
                        zsv_ext1 = zsb - 1;
                    }
                }
            } else {
                // One point on (0,0,0) side, one point on (1,1,1) side
                let c1: u8;
                let c2: u8;
                if a_is_further_side {
                    c1 = a_point;
                    c2 = b_point;
                } else {
                    c1 = b_point;
                    c2 = a_point;
                }

                // One contribution is a permutation of (1,1,-1)
                if (c1 & 0x01) == 0 {
                    dx_ext0 = dx0 + 1.0 - SQUISH_CONSTANT_3D;
                    dy_ext0 = dy0 - 1.0 - SQUISH_CONSTANT_3D;
                    dz_ext0 = dz0 - 1.0 - SQUISH_CONSTANT_3D;
                    xsv_ext0 = xsb - 1;
                    ysv_ext0 = ysb + 1;
                    zsv_ext0 = zsb + 1;
                } else if (c1 & 0x02) == 0 {
                    dx_ext0 = dx0 - 1.0 - SQUISH_CONSTANT_3D;
                    dy_ext0 = dy0 + 1.0 - SQUISH_CONSTANT_3D;
                    dz_ext0 = dz0 - 1.0 - SQUISH_CONSTANT_3D;
                    xsv_ext0 = xsb + 1;
                    ysv_ext0 = ysb - 1;
                    zsv_ext0 = zsb + 1;
                } else {
                    dx_ext0 = dx0 - 1.0 - SQUISH_CONSTANT_3D;
                    dy_ext0 = dy0 - 1.0 - SQUISH_CONSTANT_3D;
                    dz_ext0 = dz0 + 1.0 - SQUISH_CONSTANT_3D;
                    xsv_ext0 = xsb + 1;
                    ysv_ext0 = ysb + 1;
                    zsv_ext0 = zsb - 1;
                }

                // One contribution is a permutation of (0,0,2)
                dx_ext1 = dx0 - 2.0 * SQUISH_CONSTANT_3D;
                dy_ext1 = dy0 - 2.0 * SQUISH_CONSTANT_3D;
                dz_ext1 = dz0 - 2.0 * SQUISH_CONSTANT_3D;
                xsv_ext1 = xsb;
                ysv_ext1 = ysb;
                zsv_ext1 = zsb;
                if (c2 & 0x01) != 0 {
                    dx_ext1 -= 2.0;
                    xsv_ext1 += 2;
                } else if (c2 & 0x02) != 0 {
                    dy_ext1 -= 2.0;
                    ysv_ext1 += 2;
                } else {
                    dz_ext1 -= 2.0;
                    zsv_ext1 += 2;
                }
            }

            // Contribution (1,0,0)
            let dx1: f64 = dx0 - 1.0 - SQUISH_CONSTANT_3D;
            let dy1: f64 = dy0 - 0.0 - SQUISH_CONSTANT_3D;
            let dz1: f64 = dz0 - 0.0 - SQUISH_CONSTANT_3D;
            let mut attn1: f64 = 2.0 - dx1 * dx1 - dy1 * dy1 - dz1 * dz1;
            if attn1 > 0.0 {
                attn1 *= attn1;
                value +=
                    attn1 * attn1 * self.extrapolate3d(xsb + 1, ysb + 0, zsb + 0, dx1, dy1, dz1);
            }

            // Contribution (0,1,0)
            let dx2: f64 = dx0 - 0.0 - SQUISH_CONSTANT_3D;
            let dy2: f64 = dy0 - 1.0 - SQUISH_CONSTANT_3D;
            let dz2: f64 = dz1;
            let mut attn2: f64 = 2.0 - dx2 * dx2 - dy2 * dy2 - dz2 * dz2;
            if attn2 > 0.0 {
                attn2 *= attn2;
                value +=
                    attn2 * attn2 * self.extrapolate3d(xsb + 0, ysb + 1, zsb + 0, dx2, dy2, dz2);
            }

            // Contribution (0,0,1)
            let dx3: f64 = dx2;
            let dy3: f64 = dy1;
            let dz3: f64 = dz0 - 1.0 - SQUISH_CONSTANT_3D;
            let mut attn3: f64 = 2.0 - dx3 * dx3 - dy3 * dy3 - dz3 * dz3;
            if attn3 > 0.0 {
                attn3 *= attn3;
                value +=
                    attn3 * attn3 * self.extrapolate3d(xsb + 0, ysb + 0, zsb + 1, dx3, dy3, dz3);
            }

            // Contribution (1,1,0)
            let dx4: f64 = dx0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
            let dy4: f64 = dy0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
            let dz4: f64 = dz0 - 0.0 - 2.0 * SQUISH_CONSTANT_3D;
            let mut attn4: f64 = 2.0 - dx4 * dx4 - dy4 * dy4 - dz4 * dz4;
            if attn4 > 0.0 {
                attn4 *= attn4;
                value +=
                    attn4 * attn4 * self.extrapolate3d(xsb + 1, ysb + 1, zsb + 0, dx4, dy4, dz4);
            }

            // Contribution (1,0,1)
            let dx5: f64 = dx4;
            let dy5: f64 = dy0 - 0.0 - 2.0 * SQUISH_CONSTANT_3D;
            let dz5: f64 = dz0 - 1.0 - 2.0 * SQUISH_CONSTANT_3D;
            let mut attn5: f64 = 2.0 - dx5 * dx5 - dy5 * dy5 - dz5 * dz5;
            if attn5 > 0.0 {
                attn5 *= attn5;
                value +=
                    attn5 * attn5 * self.extrapolate3d(xsb + 1, ysb + 0, zsb + 1, dx5, dy5, dz5);
            }

            // Contribution (0,1,1)
            let dx6: f64 = dx0 - 0.0 - 2.0 * SQUISH_CONSTANT_3D;
            let dy6: f64 = dy4;
            let dz6: f64 = dz5;
            let mut attn6: f64 = 2.0 - dx6 * dx6 - dy6 * dy6 - dz6 * dz6;
            if attn6 > 0.0 {
                attn6 *= attn6;
                value +=
                    attn6 * attn6 * self.extrapolate3d(xsb + 0, ysb + 1, zsb + 1, dx6, dy6, dz6);
            }
        }

        // First extra vertex
        let mut attn_ext0: f64 = 2.0 - dx_ext0 * dx_ext0 - dy_ext0 * dy_ext0 - dz_ext0 * dz_ext0;
        if attn_ext0 > 0.0 {
            attn_ext0 *= attn_ext0;
            value += attn_ext0
                * attn_ext0
                * self.extrapolate3d(xsv_ext0, ysv_ext0, zsv_ext0, dx_ext0, dy_ext0, dz_ext0);
        }

        // Second extra vertex
        let mut attn_ext1: f64 = 2.0 - dx_ext1 * dx_ext1 - dy_ext1 * dy_ext1 - dz_ext1 * dz_ext1;
        if attn_ext1 > 0.0 {
            attn_ext1 *= attn_ext1;
            value += attn_ext1
                * attn_ext1
                * self.extrapolate3d(xsv_ext1, ysv_ext1, zsv_ext1, dx_ext1, dy_ext1, dz_ext1);
        }
        value
    }

    fn extrapolate2d(&self, xsb: i32, ysb: i32, dx: f64, dy: f64) -> f64 {
        let grad = &self.perm_grad2[self.perm[xsb as usize & PMASK] ^ (ysb as usize & PMASK)];
        return grad.dx * dx + grad.dy * dy;
    }
    fn extrapolate3d(&self, xsb: i32, ysb: i32, zsb: i32, dx: f64, dy: f64, dz: f64) -> f64 {
        let grad = &self.perm_grad3[self.perm
            [self.perm[xsb as usize & PMASK] ^ (ysb as usize & PMASK)]
            ^ (zsb as usize & PMASK)];
        return grad.dx * dx + grad.dy * dy + grad.dz * dz;
    }
}

#[derive(Clone, Copy)]
struct Grad2 {
    dx: f64,
    dy: f64,
}
impl Grad2 {
    const ZERO: Self = Grad2 { dx: 0.0, dy: 0.0 };
}
#[derive(Clone, Copy)]
struct Grad3 {
    dx: f64,
    dy: f64,
    dz: f64,
}
impl Grad3 {
    const ZERO: Self = Grad3 {
        dx: 0.0,
        dy: 0.0,
        dz: 0.0,
    };
}

const N2: f64 = 7.69084574549313;
const N3: f64 = 26.92263139946168;

static mut GRADIENTS_2D: [Grad2; PSIZE] = [Grad2::ZERO; PSIZE];
static mut GRADIENTS_3D: [Grad3; PSIZE] = [Grad3::ZERO; PSIZE];

pub fn init_gradients() {
    let mut grad2 = [
        Grad2 {
            dx: 0.130526192220052,
            dy: 0.99144486137381,
        },
        Grad2 {
            dx: 0.38268343236509,
            dy: 0.923879532511287,
        },
        Grad2 {
            dx: 0.608761429008721,
            dy: 0.793353340291235,
        },
        Grad2 {
            dx: 0.793353340291235,
            dy: 0.608761429008721,
        },
        Grad2 {
            dx: 0.923879532511287,
            dy: 0.38268343236509,
        },
        Grad2 {
            dx: 0.99144486137381,
            dy: 0.130526192220051,
        },
        Grad2 {
            dx: 0.99144486137381,
            dy: -0.130526192220051,
        },
        Grad2 {
            dx: 0.923879532511287,
            dy: -0.38268343236509,
        },
        Grad2 {
            dx: 0.793353340291235,
            dy: -0.60876142900872,
        },
        Grad2 {
            dx: 0.608761429008721,
            dy: -0.793353340291235,
        },
        Grad2 {
            dx: 0.38268343236509,
            dy: -0.923879532511287,
        },
        Grad2 {
            dx: 0.130526192220052,
            dy: -0.99144486137381,
        },
        Grad2 {
            dx: -0.130526192220052,
            dy: -0.99144486137381,
        },
        Grad2 {
            dx: -0.38268343236509,
            dy: -0.923879532511287,
        },
        Grad2 {
            dx: -0.608761429008721,
            dy: -0.793353340291235,
        },
        Grad2 {
            dx: -0.793353340291235,
            dy: -0.608761429008721,
        },
        Grad2 {
            dx: -0.923879532511287,
            dy: -0.38268343236509,
        },
        Grad2 {
            dx: -0.99144486137381,
            dy: -0.130526192220052,
        },
        Grad2 {
            dx: -0.99144486137381,
            dy: 0.130526192220051,
        },
        Grad2 {
            dx: -0.923879532511287,
            dy: 0.38268343236509,
        },
        Grad2 {
            dx: -0.793353340291235,
            dy: 0.608761429008721,
        },
        Grad2 {
            dx: -0.608761429008721,
            dy: 0.793353340291235,
        },
        Grad2 {
            dx: -0.38268343236509,
            dy: 0.923879532511287,
        },
        Grad2 {
            dx: -0.130526192220052,
            dy: 0.99144486137381,
        },
    ];
    for i in 0..grad2.len() {
        grad2[i].dx /= N2;
        grad2[i].dy /= N2;
    }
    for i in 0..PSIZE {
        unsafe {
            GRADIENTS_2D[i] = grad2[i % grad2.len()];
        }
    }

    let mut grad3 = [
        Grad3 {
            dx: -1.4082482904633333,
            dy: -1.4082482904633333,
            dz: -2.6329931618533333,
        },
        Grad3 {
            dx: -0.07491495712999985,
            dy: -0.07491495712999985,
            dz: -3.29965982852,
        },
        Grad3 {
            dx: 0.24732126143473554,
            dy: -1.6667938651159684,
            dz: -2.838945207362466,
        },
        Grad3 {
            dx: -1.6667938651159684,
            dy: 0.24732126143473554,
            dz: -2.838945207362466,
        },
        Grad3 {
            dx: -1.4082482904633333,
            dy: -2.6329931618533333,
            dz: -1.4082482904633333,
        },
        Grad3 {
            dx: -0.07491495712999985,
            dy: -3.29965982852,
            dz: -0.07491495712999985,
        },
        Grad3 {
            dx: -1.6667938651159684,
            dy: -2.838945207362466,
            dz: 0.24732126143473554,
        },
        Grad3 {
            dx: 0.24732126143473554,
            dy: -2.838945207362466,
            dz: -1.6667938651159684,
        },
        Grad3 {
            dx: 1.5580782047233335,
            dy: 0.33333333333333337,
            dz: -2.8914115380566665,
        },
        Grad3 {
            dx: 2.8914115380566665,
            dy: -0.33333333333333337,
            dz: -1.5580782047233335,
        },
        Grad3 {
            dx: 1.8101897177633992,
            dy: -1.2760767510338025,
            dz: -2.4482280932803,
        },
        Grad3 {
            dx: 2.4482280932803,
            dy: 1.2760767510338025,
            dz: -1.8101897177633992,
        },
        Grad3 {
            dx: 1.5580782047233335,
            dy: -2.8914115380566665,
            dz: 0.33333333333333337,
        },
        Grad3 {
            dx: 2.8914115380566665,
            dy: -1.5580782047233335,
            dz: -0.33333333333333337,
        },
        Grad3 {
            dx: 2.4482280932803,
            dy: -1.8101897177633992,
            dz: 1.2760767510338025,
        },
        Grad3 {
            dx: 1.8101897177633992,
            dy: -2.4482280932803,
            dz: -1.2760767510338025,
        },
        Grad3 {
            dx: -2.6329931618533333,
            dy: -1.4082482904633333,
            dz: -1.4082482904633333,
        },
        Grad3 {
            dx: -3.29965982852,
            dy: -0.07491495712999985,
            dz: -0.07491495712999985,
        },
        Grad3 {
            dx: -2.838945207362466,
            dy: 0.24732126143473554,
            dz: -1.6667938651159684,
        },
        Grad3 {
            dx: -2.838945207362466,
            dy: -1.6667938651159684,
            dz: 0.24732126143473554,
        },
        Grad3 {
            dx: 0.33333333333333337,
            dy: 1.5580782047233335,
            dz: -2.8914115380566665,
        },
        Grad3 {
            dx: -0.33333333333333337,
            dy: 2.8914115380566665,
            dz: -1.5580782047233335,
        },
        Grad3 {
            dx: 1.2760767510338025,
            dy: 2.4482280932803,
            dz: -1.8101897177633992,
        },
        Grad3 {
            dx: -1.2760767510338025,
            dy: 1.8101897177633992,
            dz: -2.4482280932803,
        },
        Grad3 {
            dx: 0.33333333333333337,
            dy: -2.8914115380566665,
            dz: 1.5580782047233335,
        },
        Grad3 {
            dx: -0.33333333333333337,
            dy: -1.5580782047233335,
            dz: 2.8914115380566665,
        },
        Grad3 {
            dx: -1.2760767510338025,
            dy: -2.4482280932803,
            dz: 1.8101897177633992,
        },
        Grad3 {
            dx: 1.2760767510338025,
            dy: -1.8101897177633992,
            dz: 2.4482280932803,
        },
        Grad3 {
            dx: 3.29965982852,
            dy: 0.07491495712999985,
            dz: 0.07491495712999985,
        },
        Grad3 {
            dx: 2.6329931618533333,
            dy: 1.4082482904633333,
            dz: 1.4082482904633333,
        },
        Grad3 {
            dx: 2.838945207362466,
            dy: -0.24732126143473554,
            dz: 1.6667938651159684,
        },
        Grad3 {
            dx: 2.838945207362466,
            dy: 1.6667938651159684,
            dz: -0.24732126143473554,
        },
        Grad3 {
            dx: -2.8914115380566665,
            dy: 1.5580782047233335,
            dz: 0.33333333333333337,
        },
        Grad3 {
            dx: -1.5580782047233335,
            dy: 2.8914115380566665,
            dz: -0.33333333333333337,
        },
        Grad3 {
            dx: -2.4482280932803,
            dy: 1.8101897177633992,
            dz: -1.2760767510338025,
        },
        Grad3 {
            dx: -1.8101897177633992,
            dy: 2.4482280932803,
            dz: 1.2760767510338025,
        },
        Grad3 {
            dx: -2.8914115380566665,
            dy: 0.33333333333333337,
            dz: 1.5580782047233335,
        },
        Grad3 {
            dx: -1.5580782047233335,
            dy: -0.33333333333333337,
            dz: 2.8914115380566665,
        },
        Grad3 {
            dx: -1.8101897177633992,
            dy: 1.2760767510338025,
            dz: 2.4482280932803,
        },
        Grad3 {
            dx: -2.4482280932803,
            dy: -1.2760767510338025,
            dz: 1.8101897177633992,
        },
        Grad3 {
            dx: 0.07491495712999985,
            dy: 3.29965982852,
            dz: 0.07491495712999985,
        },
        Grad3 {
            dx: 1.4082482904633333,
            dy: 2.6329931618533333,
            dz: 1.4082482904633333,
        },
        Grad3 {
            dx: 1.6667938651159684,
            dy: 2.838945207362466,
            dz: -0.24732126143473554,
        },
        Grad3 {
            dx: -0.24732126143473554,
            dy: 2.838945207362466,
            dz: 1.6667938651159684,
        },
        Grad3 {
            dx: 0.07491495712999985,
            dy: 0.07491495712999985,
            dz: 3.29965982852,
        },
        Grad3 {
            dx: 1.4082482904633333,
            dy: 1.4082482904633333,
            dz: 2.6329931618533333,
        },
        Grad3 {
            dx: -0.24732126143473554,
            dy: 1.6667938651159684,
            dz: 2.838945207362466,
        },
        Grad3 {
            dx: 1.6667938651159684,
            dy: -0.24732126143473554,
            dz: 2.838945207362466,
        },
    ];
    for i in 0..grad3.len() {
        grad3[i].dx /= N3;
        grad3[i].dy /= N3;
        grad3[i].dz /= N3;
    }
    for i in 0..PSIZE {
        unsafe {
            GRADIENTS_3D[i] = grad3[i % grad3.len()];
        }
    }
}
