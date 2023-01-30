struct Cam {
    pos: vec3<f32>,
    rot: vec3<f32>,
    inv_view_mat: mat4x4<f32>,
}
struct Proj {
    size: vec2<u32>,
    inv_mat: mat4x4<f32>,
}
struct RandSrc {
    floats: array<f32, 128>,
}
struct Settings {
    ray_dist: f32,
    water_color: vec4<f32>,
    min_water_opacity: f32,
    water_opacity_max_dist: f32,
    sky_color: vec4<f32>,
    sun_pos: vec3<f32>,
}

const CHUNK_W: u32 = 32u;
const CHUNK_H: u32 = 32u;
const CHUNK_VOLUME: u32 = 32768u; // CHUNK_W * CHUNK_H * CHUNK_W
const CHUNK_INT_COUNT: u32 = 8192u; // CHUNK_VOLUME / 4

const WORLD_W: u32 = 8u;
const WORLD_H: u32 = 8u;
const WORLD_CHUNKS_COUNT: u32 = 512u; // WORLD_W * WORLD_H * WORLD_W

const TAU: f32 = 6.283185307;

fn voxel_is_solid(voxel: u32) -> bool {
    return voxel != 0u;
}

struct Chunk {
    solid_voxels_count: u32,
    min: vec3<u32>,
    max: vec3<u32>,
    voxels: array<u32, CHUNK_INT_COUNT>,
}
struct World {
	min_chunk_pos: vec3<u32>,
	chunks: array<Chunk, WORLD_CHUNKS_COUNT>,
}

struct VoxelChunkPos {
    chunk: vec3<u32>,
    in_chunk: vec3<u32>,
}

@group(0) @binding(0) var color_buffer_: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> cam_: Cam;
@group(0) @binding(2) var<uniform> proj_: Proj;
@group(0) @binding(3) var<storage, read> world_: World;
@group(0) @binding(4) var<storage, read> rand_src_: RandSrc;
@group(0) @binding(5) var<uniform> settings_: Settings;

var<private> rand_float_idx__: i32 = 0;

const AIR: u32 = 0u;
const MAGMA: u32 = 5u;
const WATER: u32 = 6u;
const IRON: u32 = 13u;

var<private> voxel_colors__: array<vec4<f32>, 14> = array<vec4<f32>, 14>(
    vec4<f32>(0.0, 0.0, 0.0, 0.0), // AIR
    vec4<f32>(0.4, 0.4, 0.4, 1.0), // STONE
    vec4<f32>(0.4, 0.2, 0.0, 1.0), // DIRT
    vec4<f32>(0.1, 0.7, 0.1, 1.0), // GRASS
    vec4<f32>(1.0, 0.9, 0.2, 1.0), // FIRE
    vec4<f32>(0.8, 0.0, 0.0, 1.0), // MAGMA
    vec4<f32>(0.0, 0.0, 1.0, 0.2), // WATER
    vec4<f32>(1.0, 1.0, 1.0, 1.0), // WOOD TODO
    vec4<f32>(1.0, 1.0, 1.0, 1.0), // BARK TODO
    vec4<f32>(1.0, 1.0, 1.0, 1.0), // LEAVES TODO
    vec4<f32>(0.9, 0.9, 0.5, 1.0), // SAND
    vec4<f32>(1.0, 1.0, 1.0, 1.0), // MUD TODO
    vec4<f32>(1.0, 1.0, 1.0, 1.0), // CLAY TODO
    vec4<f32>(1.0, 1.0, 1.0, 1.0), // IRON
);
var<private> voxel_surface_scatter__: array<f32, 14> = array<f32, 14>(
    0.0, // AIR
    0.8, // STONE
    0.8, // DIRT
    0.8, // GRASS
    0.0, // FIRE
    0.2, // MAGMA
    0.2, // WATER
    0.8, // WOOD
    0.8, // BARK
    0.8, // LEAVES
    0.3, // SAND
    0.3, // MUD
    0.3, // CLAY
    0.0, // IRON
);

fn rand_float() -> f32 {
    let f = rand_src_.floats[rand_float_idx__];
    rand_float_idx__ += 1;
    if rand_float_idx__ >= 128 {
        rand_float_idx__ = 0;
    }
    return f;
}

fn get_chunk_voxel(chunk_idx: i32, pos: vec3<u32>) -> u32 {
    if pos.x >= CHUNK_W || pos.y >= CHUNK_H || pos.z >= CHUNK_W {
        return 101u;
    }
    let byte_idx: u32 = (pos.z * CHUNK_W * CHUNK_H) + (pos.y * CHUNK_W) + pos.x;
    
    let shift: u32 = (byte_idx % 4u) * 8u;
    let int_idx: i32 = i32(byte_idx / 4u + 1u);
    
    return (world_.chunks[chunk_idx].voxels[int_idx] >> shift) & 0xFFu;
}

fn voxel_chunk_pos(pos: vec3<u32>) -> VoxelChunkPos {
    let min_chunk_pos = world_.min_chunk_pos;
    var result: VoxelChunkPos;
    result.chunk.x = pos.x / CHUNK_W - min_chunk_pos.x;
    result.chunk.y = pos.y / CHUNK_H - min_chunk_pos.y;
    result.chunk.z = pos.z / CHUNK_W - min_chunk_pos.z;
    
    result.in_chunk.x = pos.x % CHUNK_W;
    result.in_chunk.y = pos.y % CHUNK_H;
    result.in_chunk.z = pos.z % CHUNK_W;
    return result;
}

// returns 255 when no voxel is there
fn get_world_voxel(pos: vec3<i32>) -> u32 {
    if pos.x < 0 || pos.y < 0 || pos.z < 0 {
        return 255u;
    }
    if pos.x >= i32(CHUNK_W) * i32(WORLD_W) 
    || pos.y >= i32(CHUNK_H) * i32(WORLD_H) 
    || pos.z >= i32(CHUNK_W) * i32(WORLD_W) {
        return 255u;
    }
    let pos = voxel_chunk_pos(vec3(u32(pos.x), u32(pos.y), u32(pos.z)));

    let chunk_idx = i32(pos.chunk.x + pos.chunk.y * WORLD_W + pos.chunk.z * WORLD_W * WORLD_H);
    return get_chunk_voxel(chunk_idx, pos.in_chunk);
}


struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
}
struct HitResult {
	hit: bool,
	pos: vec3<i32>,
    norm: vec3<f32>,
    exact_pos: vec3<f32>,
	face: vec3<i32>,
	voxel: u32,
    water_dist: f32,
}

fn cast_ray(ray: Ray, max_dist: f32) -> HitResult {
    let dir = ray.dir;
    let start = ray.origin;
    // -- DDA algorithm --

    // length of a line in same direction as the ray,
    // that travels 1 unit in the X, Y, Z
    let unit_step_size = vec3(
        sqrt(1.0 + (dir.y / dir.x) * (dir.y / dir.x) + (dir.z / dir.x) * (dir.z / dir.x)),
        sqrt(1.0 + (dir.x / dir.y) * (dir.x / dir.y) + (dir.z / dir.y) * (dir.z / dir.y)),
        sqrt(1.0 + (dir.x / dir.z) * (dir.x / dir.z) + (dir.y / dir.z) * (dir.y / dir.z)),
    );

    var world_pos: vec3<i32> = vec3(i32(start.x), i32(start.y), i32(start.z));
    var step: vec3<i32>;
    var ray_len: vec3<f32>;
    
    if dir.x < 0.0 {
        step.x = -1;
        ray_len.x = (start.x - f32(world_pos.x)) * unit_step_size.x;
    } else {
        step.x = 1;
        ray_len.x = (f32(world_pos.x + 1) - start.x) * unit_step_size.x;
    }
    if dir.y < 0.0 {
    	step.y = -1;
    	ray_len.y = (start.y - f32(world_pos.y)) * unit_step_size.y;
    } else {
    	step.y = 1;
        ray_len.y = (f32(world_pos.y + 1) - start.y) * unit_step_size.y;
    }
    if dir.z < 0.0 {
        step.z = -1;
        ray_len.z = (start.z - f32(world_pos.z)) * unit_step_size.z;
    } else {
        step.z = 1;
        ray_len.z = (f32(world_pos.z + 1) - start.z) * unit_step_size.z;
    }
    
    let min_chunk_pos = world_.min_chunk_pos;
    
    var dist: f32 = 0.0;
    var prev_world_pos: vec3<i32>;
    var result: HitResult;
    
    // the distance the ray travled when it entered water
    var dist_entered_water: f32 = -1.0;

    while dist < max_dist {
        prev_world_pos = world_pos;
        // walk
        if ray_len.x < ray_len.y && ray_len.x < ray_len.z {
            world_pos.x += step.x;
            dist = ray_len.x;
            ray_len.x += unit_step_size.x;
        } else if ray_len.z < ray_len.x && ray_len.z < ray_len.y {
            world_pos.z += step.z;
            dist = ray_len.z;
            ray_len.z += unit_step_size.z;
        } else {
            world_pos.y += step.y;
            dist = ray_len.y;
            ray_len.y += unit_step_size.y;
        }
        
        // if the position of the voxel is outside the world, skip
        if world_pos.x < 0 || world_pos.y < 0 || world_pos.z < 0 
        || world_pos.x >= i32(CHUNK_W) * i32(WORLD_W)
        || world_pos.y >= i32(CHUNK_H) * i32(WORLD_H)
        || world_pos.z >= i32(CHUNK_W) * i32(WORLD_W) {
            continue;
        }
        
        // calculate the position of the chunk that contains `world_pos`
        var chunk: vec3<i32>;
        var in_chunk: vec3<u32>;
        chunk.x = world_pos.x / i32(CHUNK_W) - i32(min_chunk_pos.x);
        chunk.y = world_pos.y / i32(CHUNK_H) - i32(min_chunk_pos.y);
        chunk.z = world_pos.z / i32(CHUNK_W) - i32(min_chunk_pos.z);
        let chunk_idx = chunk.x + chunk.y * i32(WORLD_W) + chunk.z * i32(WORLD_W) * i32(WORLD_H);
        
        // if the chunk is empty, the voxel isn't solid, skip
        // TODO skip entire chunk, not just this voxel
        if world_.chunks[chunk_idx].solid_voxels_count == 0u {
            continue;
        }
        
        // TODO 
        // if the ray doesnt hit the bounding box for the min/max voxels in the chunk, skip the chunk
        
        // calculate the position of `world_pos` in the chunk
        in_chunk.x = u32(world_pos.x) % CHUNK_W;
        in_chunk.y = u32(world_pos.y) % CHUNK_H;
        in_chunk.z = u32(world_pos.z) % CHUNK_W;

        let voxel = get_chunk_voxel(chunk_idx, in_chunk);
        
        // make sure the voxel is solid
        if voxel == AIR {
            continue;
        }
        if voxel != WATER {
            if dist_entered_water != -1.0 {
                result.water_dist += dist - dist_entered_water;
                dist_entered_water = -1.0;
            }
        }
        if voxel == WATER {
            if dist_entered_water == -1.0 {
                dist_entered_water = dist;
            }
            continue;
        }
        
        // done
        result.voxel = voxel;
        result.pos = world_pos;
        result.hit = true;
        result.face = prev_world_pos - result.pos;
        result.norm = vec3(f32(result.face.x), f32(result.face.y), f32(result.face.z));
        result.exact_pos = start + dir * dist;
        return result;
    }
    if dist_entered_water != -1.0 {
        result.water_dist += dist - dist_entered_water;
    }
    return result;
}
fn create_ray_from_screen(screen_pos: vec2<i32>) -> Ray {
	let x = (f32(screen_pos.x) * 2.0) / f32(proj_.size.x) - 1.0;
	let y = (f32(screen_pos.y) * 2.0) / f32(proj_.size.y) - 1.0;
	let clip_coords = vec4(x, -y, -1.0, 1.0);
	let eye_coords0 = clip_coords * proj_.inv_mat;
	let eye_coords = vec4(eye_coords0.xy, -1.0, 0.0);
	let ray_world = normalize((eye_coords * cam_.inv_view_mat).xyz);
	
	var ray: Ray;
	ray.origin = cam_.pos;
	ray.dir = ray_world;
	return ray;
}

struct VertexOutput {
	@builtin(position) clip_pos: vec4<f32>,
}

fn overlay_color(back: vec4<f32>, front: vec4<f32>, factor: f32) -> vec4<f32> {
    return back * (1.0 - factor) + front * factor;
}

@compute @workgroup_size(1, 1, 1)
fn update(@builtin(global_invocation_id) inv_id: vec3<u32>) {
    let screen_pos: vec2<i32> = vec2(i32(inv_id.x), i32(inv_id.y));

	let ray = create_ray_from_screen(screen_pos);
	var result = cast_ray(ray, settings_.ray_dist);
    
    var color: vec4<f32> = settings_.sky_color;
    if result.hit {
        var iron_count = 0;
        while result.voxel == IRON && iron_count < 5 {
            var reflect: Ray;
            reflect.dir = ray.dir - 2.0 * result.norm * dot(result.norm, ray.dir);
            reflect.origin = result.exact_pos + reflect.dir * 0.2;
        
            result = cast_ray(reflect, settings_.ray_dist);
            iron_count += 1;
        }
        
        color = voxel_colors__[i32(result.voxel)];
        if iron_count > 0 {
            let factor = min(f32(iron_count) / 5.0, 1.0);
            color = overlay_color(color, vec4(1.0), factor);
        }
        
        // var reflect: Ray;
        // reflect.dir = ray.dir - 2.0 * norm * dot(norm, ray.dir);
        // reflect.origin = result.exact_pos + reflect.dir * 0.2;
        
        // let reflect_hit = cast_ray(reflect, 20.0);
        
        var to_sun: Ray;
        to_sun.dir = normalize(settings_.sun_pos - result.exact_pos);
        to_sun.origin = result.exact_pos + to_sun.dir * 0.001;
        let to_sun_result = cast_ray(to_sun, 50.0);
        if to_sun_result.hit {
            color *= 0.9;
        }
        
        if result.face.x ==  1 { color *= 0.7; }
        if result.face.x == -1 { color *= 0.7; }
        if result.face.z ==  1 { color *= 0.8; }
        if result.face.z == -1 { color *= 0.8; }
        if result.face.y ==  1 { color *= 1.0; }
        if result.face.y == -1 { color *= 0.3; }
    }
    
    if result.water_dist != 0.0 {
        var factor = clamp(
            result.water_dist / settings_.water_opacity_max_dist, 
            settings_.min_water_opacity, 1.0
        );
    
        color = overlay_color(color, settings_.water_color, factor);
    }
    
    textureStore(color_buffer_, screen_pos, color);
}
