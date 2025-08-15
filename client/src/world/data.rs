pub static VOXEL_NAMES: &[&str] = &[
    "Air",
    "Stone",
    "Dirt",
    "Grass",
    "Snow",
    "Dead Grass",
    "Moist Grass",
    "Sand",
    "Mud",
    "Clay",
    "Fire",
    "Magma",
    "Water",
    "Oak Wood",
    "Oak Leaves",
    "Birch Wood",
    "Birch Leaves",
    "Spruce Wood",
    "Spruce Leaves",
    "Cactus",
    "Gold",
    "Mirror",
    "Bright",
];

pub static VOXEL_MATERIALS: &[Material] = &[
    Material::empty(),                                            // Air
    Material::solid([0.40, 0.40, 0.40], 1.0),                     // Stone
    Material::solid([0.40, 0.20, 0.00], 1.0),                     // Dirt
    Material::solid([0.011, 0.58, 0.11], 1.0),                    // Grass
    Material::solid([1.0; 3], 0.8),                               // Snow
    Material::solid([0.2, 0.4, 0.2], 1.0),                        // Dead Grass
    Material::solid([1.0, 0.0, 0.0], 1.0),                        // Moist Grass
    Material::solid([1.00, 0.9, 0.3], 0.9),                       // Sand
    Material::solid([0.22, 0.13, 0.02], 0.8),                     // Mud
    Material::solid([0.35, 0.30, 0.25], 0.8),                     // Clay
    Material::solid([1.00, 0.90, 0.20], 0.0).emit(2.0),           // Fire
    Material::solid([0.75, 0.18, 0.01], 1.0).emit(1.0),           // Magma
    Material::solid([0.076, 0.563, 0.563], 0.0).translucent(0.7), // Water
    Material::solid([0.25, 0.10, 0.00], 1.0),                     // Oak Wood
    Material::solid([0.23, 0.52, 0.00], 1.0),                     // Oak Leaves
    Material::solid([1.0; 3], 1.0),                               // Birch Wood
    Material::solid([0.43, 0.72, 0.00], 1.0),                     // Birch Leaves
    Material::solid([0.06, 0.04, 0.00], 1.0),                     // Spruce Wood
    Material::solid([0.04, 0.22, 0.00], 1.0),                     // Spruce Leaves
    Material::solid([0.0, 0.30, 0.0], 1.0),                       // Cactus
    Material::solid([0.83, 0.68, 0.22], 0.3),                     // Gold
    Material::solid([1.0; 3], 0.0),                               // Mirror
    Material::solid([1.0; 3], 1.0).emit(5.0),                     // Bright
];

#[derive(Clone)]
#[repr(C)]
pub struct Material {
    pub color: [f32; 3],
    pub empty: u32,
    pub scatter: f32,
    pub emission: f32,
    pub polish_bounce_chance: f32,
    pub translucency: f32,
    pub polish_color: [f32; 3],
    pub polish_scatter: f32,
}
impl Material {
    pub const ZERO: Self = Self {
        color: [0.0; 3],
        empty: 0,
        scatter: 0.0,
        emission: 0.0,
        polish_bounce_chance: 0.0,
        translucency: 0.0,
        polish_color: [0.0; 3],
        polish_scatter: 0.0,
    };

    pub const fn empty() -> Self {
        let mut rs = Self::ZERO;
        rs.empty = 1;
        rs
    }

    pub const fn solid(color: [f32; 3], scatter: f32) -> Self {
        let mut rs = Self::ZERO;
        rs.color = color;
        rs.scatter = scatter;
        rs
    }

    pub const fn translucent(mut self, t: f32) -> Self {
        self.translucency = t;
        self
    }

    pub const fn polished(mut self, chance: f32, scatter: f32, color: [f32; 3]) -> Self {
        self.polish_bounce_chance = chance;
        self.polish_color = color;
        self.polish_scatter = scatter;
        self
    }

    pub const fn emit(mut self, emission: f32) -> Self {
        self.emission = emission;
        self
    }
}
