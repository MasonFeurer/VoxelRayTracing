use super::{
    noise::NoiseMap, ChunkHeader, FoundNode, Node, NodeAlloc, Voxel, World, WorldErr, CHUNK_DEPTH,
    CHUNK_SIZE,
};
use crate::math::{rand_cardinal_dir, rand_hem_dir};
use glam::{ivec3, uvec3, vec2, IVec3, Vec3};
use std::{ops::Range, sync::mpsc::Sender};

fn randf32(range: Range<f32>) -> f32 {
    let size = range.end - range.start;
    fastrand::f32() * size + range.start
}

struct NoiseMaps {
    height: NoiseMap,
    freq: NoiseMap,
    scale: NoiseMap,
    bumps: NoiseMap,
    mountains: NoiseMap,
    temp: NoiseMap,
    moisture: NoiseMap,
    vegetation: NoiseMap,
}
impl NoiseMaps {
    pub fn from_seed(seed: i64) -> Self {
        Self {
            height: NoiseMap::new(seed.wrapping_mul(4326742), 0.003, 2.5),
            freq: NoiseMap::new(seed.wrapping_mul(927144), 0.0001, 7.0),
            scale: NoiseMap::new(seed.wrapping_mul(43265), 0.003, 40.0),
            bumps: NoiseMap::new(seed.wrapping_mul(76324), 0.15, 4.0),
            mountains: NoiseMap::new(seed.wrapping_mul(72316423), 0.001, 40.0),
            temp: NoiseMap::new(seed.wrapping_mul(83226), 0.0004, 1.0),
            moisture: NoiseMap::new(seed.wrapping_mul(2345632), 0.0004, 1.0),
            vegetation: NoiseMap::new(seed.wrapping_mul(53252), 0.001, 1.0),
        }
    }
}

fn find_node_in_region(nodes: &[Node], pos: IVec3) -> Result<FoundNode, WorldErr> {
    let mut center = IVec3::splat(CHUNK_SIZE as i32 / 2);
    let mut size = CHUNK_SIZE;
    let mut idx: u32 = 0;
    let mut depth: u32 = 0;

    loop {
        let node = nodes[idx as usize];
        if !node.is_split() || depth == CHUNK_DEPTH {
            return Ok(FoundNode {
                idx,
                depth,
                center,
                size,
            });
        }
        size /= 2;

        let gt = ivec3(
            (pos.x >= center.x) as i32,
            (pos.y >= center.y) as i32,
            (pos.z >= center.z) as i32,
        );
        let child_idx = ((gt.x as u32) << 0) | ((gt.y as u32) << 1) | ((gt.z as u32) << 2);
        idx = node.child_idx() + child_idx;
        let child_dir = gt * 2 - IVec3::ONE;
        center += IVec3::splat(size as i32 / 2) * child_dir;
        depth += 1;
    }
}

fn set_voxel_in_region(
    alloc: &mut NodeAlloc,
    nodes: &mut [Node],
    pos: IVec3,
    voxel: Voxel,
) -> Result<(), WorldErr> {
    let FoundNode {
        mut idx,
        mut center,
        mut size,
        depth,
        ..
    } = find_node_in_region(nodes, pos)?;

    let parent_voxel = nodes[idx as usize].voxel();

    // If depth is less than target_depth,
    // the SVO doesn't go to desired depth, so we must split until it does
    for _ in depth..CHUNK_DEPTH {
        // note: allocators don't move, so alloc_idx will always be valid
        let first_child = alloc.next()?;
        for i in first_child..(first_child + 8) {
            nodes[i as usize] = Node::new(parent_voxel);
        }
        nodes[idx as usize] = Node::new_split(first_child);
        size /= 2;

        let gt = ivec3(
            (pos.x >= center.x) as i32,
            (pos.y >= center.y) as i32,
            (pos.z >= center.z) as i32,
        );
        let child_idx = (gt.x as u32) << 0 | (gt.y as u32) << 1 | (gt.z as u32) << 2;
        idx = first_child + child_idx;
        let child_dir = gt * 2 - IVec3::ONE;
        center += IVec3::splat(size as i32 / 2) * child_dir;
    }
    // SVO now goes to desired depth, so we can mutate the node now.
    nodes[idx as usize] = Node::new(voxel);
    Ok(())
}

pub struct WorldGen {
    maps: NoiseMaps,
    seed: i64,
    oak_tree_gen: TreeGen,
    birch_tree_gen: TreeGen,
    spruce_tree_gen: SpruceTreeGen,
    cactus_gen: CactusGen,
}
impl WorldGen {
    pub fn new(seed: i64) -> Self {
        let oak_tree_gen = TreeGen {
            height: 6..19,
            wood_voxel: Voxel::OAK_WOOD,
            leaves_voxel: Voxel::OAK_LEAVES,
            leaves_decay: 0.1,
            branch_count: 1..4,
            branch_height: 0.5..0.8,
            branch_len: 3.0..8.0,
        };
        let birch_tree_gen = TreeGen {
            height: 9..26,
            wood_voxel: Voxel::BIRCH_WOOD,
            leaves_voxel: Voxel::BIRCH_LEAVES,
            leaves_decay: 0.1,
            branch_count: 1..4,
            branch_height: 0.5..0.8,
            branch_len: 3.0..8.0,
        };
        let spruce_tree_gen = SpruceTreeGen {
            height: 10..18,
            bottom_branch: 4..8,
        };
        let cactus_gen = CactusGen { height: 2..7 };
        let maps = NoiseMaps::from_seed(seed);
        Self {
            maps,
            seed,
            birch_tree_gen,
            oak_tree_gen,
            spruce_tree_gen,
            cactus_gen,
        }
    }

    pub fn seed(&self) -> i64 {
        self.seed
    }

    pub fn chunk_voxel(&self, min: IVec3, max: IVec3) -> Option<Voxel> {
        if max.y < 0 {
            return Some(Voxel::STONE);
        }
        if min.y > 128 {
            return Some(Voxel::AIR);
        }
        None
    }

    pub fn sample_terrain(&self, pos: IVec3) -> (Voxel, bool) {
        let pos2 = vec2(pos.x as f32, pos.z as f32);
        let h_freq = self.maps.freq.get(pos2);
        let h_scale = self.maps.scale.get(pos2);
        let h = (self.maps.height.get(pos2 * h_freq) * h_scale
            + self.maps.bumps.get(pos2)
            + self.maps.mountains.get(pos2)) as i32;

        if h < 26 {
            return (
                match pos.y {
                    v if v < h - 4 => Voxel::STONE,
                    v if v < h => Voxel::DIRT,
                    v if v == h => Voxel::SAND,
                    v if v < 26 => Voxel::WATER,
                    _ => Voxel::AIR,
                },
                false,
            );
        } else {
            if pos.y < h - 4 {
                return (Voxel::STONE, false);
            }
            if pos.y < h {
                return (Voxel::DIRT, false);
            }
            if pos.y > h {
                return (Voxel::AIR, false);
            }
        }

        let temp = self.maps.temp.get(pos2);
        let moisture = self.maps.moisture.get(pos2);

        (
            match (moisture, temp) {
                (m, t) if m < 0.3 && t > 0.7 => Voxel::SAND,
                (m, t) if m < 0.3 && t < 0.3 => Voxel::DEAD_GRASS,
                (m, t) if m > 0.3 && t < 0.3 => Voxel::SNOW,
                (m, t) if m > 0.7 && t > 0.7 => Voxel::MOIST_GRASS,
                _ => Voxel::GRASS,
            },
            true,
        )
    }

    pub fn build_chunk2(
        &self,
        origin: IVec3,
        alloc: &mut NodeAlloc,
        nodes: &mut [Node],
        features: Sender<Feature>,
    ) -> Result<(), WorldErr> {
        for x in 0i32..CHUNK_SIZE as i32 {
            for z in 0i32..CHUNK_SIZE as i32 {
                let world_xz = glam::ivec2(x + origin.x, z + origin.z);
                let vegetation = self.maps.vegetation.get(world_xz.as_vec2());

                for y in 0i32..CHUNK_SIZE as i32 {
                    let (world_pos, local_pos) = (ivec3(x, y, z) + origin, ivec3(x, y, z));

                    let (voxel, is_surface) = self.sample_terrain(world_pos);
                    if voxel == Voxel::AIR {
                        // if the sampler returned air,
                        // then there arn't going to be any more solid blocks
                        break;
                    }
                    set_voxel_in_region(alloc, nodes, local_pos, voxel)?;

                    if !is_surface {
                        continue;
                    }

                    if voxel == Voxel::GRASS && fastrand::f32() < 0.005 * vegetation {
                        match fastrand::u8(0..2) {
                            0 => _ = features.send(self.oak_tree_gen.generate(world_pos)),
                            1 => _ = features.send(self.birch_tree_gen.generate(world_pos)),
                            _ => unreachable!(),
                        }
                    }
                    if voxel == Voxel::SAND && fastrand::f32() < 0.01 * vegetation {
                        _ = features.send(self.cactus_gen.generate(world_pos));
                    }
                    if voxel == Voxel::SNOW && fastrand::f32() < 0.003 * vegetation {
                        _ = features.send(self.spruce_tree_gen.generate(world_pos));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn build_chunk(
        &self,
        chunk: ChunkHeader,
        min: IVec3,
        world: &mut World,
        features: Sender<Feature>,
    ) -> Result<(), WorldErr> {
        for x in 0i32..CHUNK_SIZE as i32 {
            for z in 0i32..CHUNK_SIZE as i32 {
                let world_xz = glam::ivec2(x + min.x, z + min.z);
                let vegetation = self.maps.vegetation.get(world_xz.as_vec2());

                for y in 0i32..CHUNK_SIZE as i32 {
                    let (world_pos, local_pos) = (ivec3(x, y, z) + min, ivec3(x, y, z));

                    let (voxel, is_surface) = self.sample_terrain(world_pos);
                    if voxel == Voxel::AIR {
                        // if the sampler returned air,
                        // then there arn't going to be any more solid blocks
                        break;
                    }
                    world.set_voxel_in_chunk(chunk.clone(), local_pos, voxel, |_| {})?;

                    if !is_surface {
                        continue;
                    }

                    if voxel == Voxel::GRASS && fastrand::f32() < 0.005 * vegetation {
                        match fastrand::u8(0..2) {
                            0 => _ = features.send(self.oak_tree_gen.generate(world_pos)),
                            1 => _ = features.send(self.birch_tree_gen.generate(world_pos)),
                            _ => unreachable!(),
                        }
                    }
                    if voxel == Voxel::SAND && fastrand::f32() < 0.01 * vegetation {
                        _ = features.send(self.cactus_gen.generate(world_pos));
                    }
                    if voxel == Voxel::SNOW && fastrand::f32() < 0.003 * vegetation {
                        _ = features.send(self.spruce_tree_gen.generate(world_pos));
                    }
                }
            }
        }
        Ok(())
    }
}

enum Shape {
    Line {
        points: [IVec3; 2],
        voxel: Voxel,
    },
    Sphere {
        center: IVec3,
        r: u32,
        voxel: Voxel,
    },
    Cube {
        center: IVec3,
        r: u32,
        voxel: Voxel,
    },
    Disc {
        center: IVec3,
        r: u32,
        height: u32,
        voxel: Voxel,
    },
    Voxel {
        pos: IVec3,
        voxel: Voxel,
    },
}
impl Shape {
    fn bounds(&self) -> [IVec3; 2] {
        match self {
            Self::Voxel { pos, .. } => [*pos, *pos],
            Self::Cube { center, r, .. } => [
                *center - IVec3::splat(*r as i32 + 1),
                *center + IVec3::splat(*r as i32 + 1),
            ],
            Self::Sphere { center, r, .. } => [
                *center - IVec3::splat(*r as i32 + 1),
                *center + IVec3::splat(*r as i32 + 1),
            ],
            Self::Line { points, .. } => [
                points[0].min(points[1]) - IVec3::ONE,
                points[0].max(points[1]) + IVec3::ONE,
            ],
            Self::Disc {
                center, r, height, ..
            } => [
                *center - ivec3(*r as i32, 0, *r as i32),
                *center + uvec3(*r, *height, *r).as_ivec3(),
            ],
        }
    }

    fn place(&self, h: &mut impl FnMut(IVec3, Voxel)) {
        match self {
            Self::Voxel { pos, voxel } => h(*pos, *voxel),
            Self::Line { points, voxel } => {
                for pos in crate::math::walk_line(points[0], points[1]) {
                    h(pos, *voxel)
                }
            }
            Self::Sphere { center, r, voxel } => {
                let pos_center = center.as_vec3() + Vec3::splat(0.5);
                let min = *center - IVec3::splat(*r as i32);
                let max = *center + IVec3::splat(*r as i32);
                let r_sq = *r as f32 * *r as f32;

                for x in min.x..=max.x {
                    for y in min.y..=max.y {
                        for z in min.z..=max.z {
                            let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                            let dist_sq = (block_center - pos_center).length_squared();

                            if dist_sq >= r_sq {
                                continue;
                            }
                            h(ivec3(x, y, z), *voxel);
                        }
                    }
                }
            }
            Self::Cube { center, r, voxel } => {
                let min = *center - IVec3::splat(*r as i32);
                let max = *center + IVec3::splat(*r as i32);
                for x in min.x..max.x {
                    for y in min.y..max.y {
                        for z in min.z..max.z {
                            h(ivec3(x, y, z), *voxel);
                        }
                    }
                }
            }
            Self::Disc {
                center,
                r,
                height,
                voxel,
            } => {
                let pos_center = center.as_vec3() + Vec3::splat(0.5);
                let min = *center - ivec3(*r as i32, 0, *r as i32);
                let max = *center + ivec3(*r as i32, *height as i32 - 1, *r as i32);
                let r_sq = *r as f32 * *r as f32;

                for x in min.x..=max.x {
                    for y in min.y..=max.y {
                        for z in min.z..=max.z {
                            let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                            let dist_sq = (block_center - pos_center).length_squared();

                            if dist_sq >= r_sq {
                                continue;
                            }
                            h(ivec3(x, y, z), *voxel);
                        }
                    }
                }
            }
        }
    }
}

pub struct Feature {
    bounds: [IVec3; 2],
    shapes: Vec<Shape>,
}
impl Default for Feature {
    fn default() -> Self {
        Self {
            bounds: [IVec3::MAX, IVec3::MIN],
            shapes: vec![],
        }
    }
}
impl Feature {
    fn push_shape(&mut self, shape: Shape) {
        let [min, max] = shape.bounds();
        self.bounds[0] = self.bounds[0].min(min);
        self.bounds[1] = self.bounds[1].max(max);
        self.shapes.push(shape);
    }

    #[inline(always)]
    pub fn min(&self) -> IVec3 {
        self.bounds[0]
    }
    #[inline(always)]
    pub fn max(&self) -> IVec3 {
        self.bounds[1]
    }

    pub fn sphere(&mut self, center: IVec3, r: u32, voxel: Voxel, _decay: f32) {
        self.push_shape(Shape::Sphere { center, r, voxel })
    }
    #[rustfmt::skip]
    pub fn disc(&mut self, center: IVec3, r: u32, height: u32, voxel: Voxel) {
        self.push_shape(Shape::Disc { center, r, height, voxel })
    }
    pub fn line(&mut self, points: [IVec3; 2], voxel: Voxel) {
        self.push_shape(Shape::Line { points, voxel })
    }
    pub fn cube(&mut self, center: IVec3, r: u32, voxel: Voxel) {
        self.push_shape(Shape::Cube { center, r, voxel })
    }
    pub fn voxel(&mut self, pos: IVec3, voxel: Voxel) {
        self.push_shape(Shape::Voxel { pos, voxel })
    }

    pub fn place(&self, mut h: impl FnMut(IVec3, Voxel)) {
        for shape in &self.shapes {
            shape.place(&mut h);
        }
    }
}

#[derive(Clone)]
pub struct TreeGen {
    pub height: Range<u32>,
    pub wood_voxel: Voxel,
    pub leaves_voxel: Voxel,
    pub leaves_decay: f32,
    pub branch_count: Range<u32>,
    pub branch_height: Range<f32>,
    pub branch_len: Range<f32>,
}
impl TreeGen {
    fn generate(&self, surface: IVec3) -> Feature {
        let mut rs = Feature::default();
        let height = fastrand::u32(self.height.clone());
        let top = surface + ivec3(0, height as i32, 0);

        let branch_count = match height {
            ..=8 => 0,
            _ => fastrand::u32(self.branch_count.clone()),
        };
        rs.sphere(top, 5, self.leaves_voxel, self.leaves_decay);

        for _ in 0..branch_count {
            let branch_h = (randf32(self.branch_height.clone()) * height as f32) as u32;
            let branch_len = randf32(self.branch_len.clone());

            let branch_dir = rand_hem_dir(Vec3::Y);
            let start = ivec3(surface.x, surface.y + branch_h as i32, surface.z);
            let end = (start.as_vec3() + branch_dir * branch_len).as_ivec3();

            rs.sphere(end, 3, self.leaves_voxel, self.leaves_decay);
            rs.line([start, end], self.wood_voxel);
        }
        rs.line([surface, top], self.wood_voxel);
        rs
    }
}

#[derive(Clone)]
struct SpruceTreeGen {
    height: Range<u32>,
    bottom_branch: Range<u32>,
}
impl SpruceTreeGen {
    fn generate(&self, pos: IVec3) -> Feature {
        let mut rs = Feature::default();
        let offset = fastrand::u32(self.bottom_branch.clone()) as i32;
        let height = offset + fastrand::u32(self.height.clone()) as i32;

        let mut y = height;
        let mut r: i32 = 1;
        while y > offset {
            let c = pos + IVec3::Y * y;
            rs.disc(c, r as u32, 1, Voxel::SPRUCE_LEAVES);
            r += 1;
            y -= 2;
        }
        let top = pos + ivec3(0, height - 1, 0);
        rs.line([pos, top], Voxel::SPRUCE_WOOD);
        rs
    }
}

#[derive(Clone)]
struct CactusGen {
    height: Range<u32>,
}
impl CactusGen {
    fn generate(&self, pos: IVec3) -> Feature {
        let mut rs = Feature::default();
        let pos = pos + IVec3::Y;
        let height = fastrand::u32(self.height.clone()) as i32;
        let splits = if height > 3 { fastrand::u32(0..4) } else { 0 };

        rs.line([pos, pos + IVec3::Y * height], Voxel::CACTUS);
        for _ in 0..splits {
            let split_h = fastrand::i32(1..height);
            let split_len = fastrand::i32(1..4);
            let dir = rand_cardinal_dir();

            rs.voxel(pos + IVec3::Y * split_h + dir, Voxel::CACTUS);
            let branch_min = pos + IVec3::Y * split_h + dir * 2;
            let branch_max = branch_min + IVec3::Y * split_len;
            rs.line([branch_min, branch_max], Voxel::CACTUS);
        }
        rs
    }
}
