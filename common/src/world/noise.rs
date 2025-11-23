use glam::Vec2;
use noise::core::perlin::perlin_2d;
use noise::permutationtable::PermutationTable;
use serde::Deserialize;

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct Map {
    pub freq: f32,
    pub scale: f32,
    pub offset: f32,
}
impl Map {
    pub fn new(freq: f32, scale: f32, offset: f32) -> Self {
        Self {
            freq,
            scale,
            offset,
        }
    }
}

#[derive(Clone)]
pub struct RawNoise {
    table: Box<PermutationTable>,
}
impl RawNoise {
    pub fn new(seed: i64) -> Self {
        Self {
            table: Box::new(PermutationTable::new(seed as u32)),
        }
    }

    // Will return a value from 0.0 to 1.0
    pub fn sample(&self, pos: Vec2) -> f32 {
        let val = perlin_2d((pos.x as f64, pos.y as f64).into(), &*self.table);
        // val is -1.0 ..= 1.0, we need it to be 0.0 .. 1.0
        ((val + 1.0) * 0.5).clamp(0.0, 1.0) as f32
    }

    pub fn map_sample(&self, pos: Vec2, map: &Map) -> f32 {
        (self.sample(pos * map.freq) * map.scale + map.offset) as f32
    }
}

#[derive(Clone)]
pub struct MappedNoise {
    pub raw: RawNoise,
    pub map: Map,
}
impl MappedNoise {
    pub fn new(seed: i64, map: Map) -> Self {
        Self {
            raw: RawNoise::new(seed),
            map,
        }
    }

    // Will return a value from 0.0 to 1.0, multiplied by `self.scale`, added to `self.offset`
    pub fn sample(&self, pos: Vec2) -> f32 {
        (self.raw.sample(pos * self.map.freq) * self.map.scale + self.map.offset) as f32
    }
}
