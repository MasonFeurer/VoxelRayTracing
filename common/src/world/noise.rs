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
    pub fn get(&self, pos: Vec2) -> f32 {
        let val = perlin_2d(
            (pos.x as f64 * self.freq, pos.y as f64 * self.freq).into(),
            &*self.table,
        );

        // let val = self
        //     .noise
        //     .eval2d(pos.x as f64 * self.freq, pos.y as f64 * self.freq);
        (((val + 1.0) * 0.5) * self.scale + self.offset) as f32
    }
}
