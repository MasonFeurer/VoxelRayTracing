use glam::Vec2;

use noise::core::perlin::perlin_2d;
use noise::permutationtable::PermutationTable;

#[derive(Clone)]
pub struct NoiseMap {
    table: Box<PermutationTable>,
    pub scale: f64,
    pub freq: f64,
    pub offset: f64,
}
impl NoiseMap {
    pub fn new(seed: i64, freq: f64, scale: f64, offset: f64) -> Self {
        Self {
            table: Box::new(PermutationTable::new(seed as u32)),
            freq,
            scale,
            offset,
        }
    }

    pub fn update(&mut self, freq: f64, scale: f64, offset: f64) {
        self.freq = freq;
        self.scale = scale;
        self.offset = offset;
    }

    // Will return a value from 0.0 to 1.0, multiplied by `self.scale`, added to `self.offset`
    pub fn get(&self, pos: Vec2) -> f32 {
        let val = perlin_2d(
            (pos.x as f64 * self.freq, pos.y as f64 * self.freq).into(),
            &*self.table,
        );
        // val is -1.0 ..= 1.0, we need it to be 0.0 .. 1.0
        let shifted_val = ((val + 1.0) * 0.5).clamp(0.0, 1.0);

        (shifted_val * self.scale + self.offset) as f32
    }
}
