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
    max_ray_steps: u32,
    water_color: vec4<f32>,
    min_water_opacity: f32,
    water_opacity_max_dist: f32,
    sky_color: vec4<f32>,
    sun_pos: vec3<f32>,
    max_reflections: u32,
    shadows: u32, // bool
    ray_cast_method: u32,
}

const WORLD_W: u32 = 256u;
const WORLD_H: u32 = 256u;
const WORLD_VOLUME: u32 = 16777216u; // WORLD_W * WORLD_H * WORLD_W
const WORLD_INT_COUNT: u32 = 4194304u; // WORLD_VOLUME / 4

// reference: https://www.w3.org/TR/WGSL/#structure-member-layout
struct World {
    origin: vec3<u32>,
    filler: u32, // no purpose but to align `voxels` to 16 bytes
    voxels: array<u32, WORLD_INT_COUNT>,
}

@group(0) @binding(0) var color_buffer_: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> cam_: Cam;
@group(0) @binding(2) var<uniform> proj_: Proj;
@group(0) @binding(3) var<storage, read> world_: World;
@group(0) @binding(4) var<storage, read> rand_src_: RandSrc;
@group(0) @binding(5) var<uniform> settings_: Settings;
// @group(0) @binding(6) var sdf: texture_2d<u32>;
@group(0) @binding(6) var<storage, read> sdf_: array<u32, WORLD_INT_COUNT>;

var<private> rand_float_idx__: i32 = 0;

const WATER: u32 = 6u;

var<private> voxel_colors__: array<vec4<f32>, 15> = array<vec4<f32>, 15>(
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
    vec4<f32>(1.0, 1.0, 0.0, 1.0), // GOLD
    vec4<f32>(1.0, 1.0, 1.0, 1.0), // MIRROR
);
var<private> voxel_surface_scatter__: array<f32, 15> = array<f32, 15>(
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
    0.0, // GOLD
    0.0, // MIRROR
);
var<private> voxel_reflection__: array<f32, 15> = array<f32, 15>(
    0.0, // AIR
    0.0, // STONE
    0.0, // DIRT
    0.0, // GRASS
    0.0, // FIRE
    0.0, // MAGMA
    0.0, // WATER
    0.0, // WOOD
    0.0, // BARK
    0.0, // LEAVES
    0.0, // SAND
    0.0, // MUD
    0.0, // CLAY
    0.5, // GOLD
    1.0, // MIRROR
);

fn rand_float() -> f32 {
    let f = rand_src_.floats[rand_float_idx__];
    rand_float_idx__ += 1;
    if rand_float_idx__ >= 128 {
        rand_float_idx__ = 0;
    }
    return f;
}

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
}
struct HitResult {
	hit: bool,
    color: vec4<f32>,
    norm: vec3<f32>,
    pos: vec3<f32>,
    reflection: f32,
}

fn cast_ray2(ray: Ray, max_dist: f32) -> HitResult {
    let origin = ray.origin;
    let dir = ray.dir;
    var result: HitResult;
    result.color = settings_.sky_color;
    
    let inv_dir = 1.0 / dir;

    let delta = abs(inv_dir);
    let sray: vec3<i32> = vec3<i32>(sign(dir));
    let dray: vec3<f32> = vec3<f32>(sray) * inv_dir;
    let add: vec3<f32> = floor(vec3<f32>(sray));
    var world_pos: vec3<i32> = vec3<i32>(floor(origin));
    var dist: vec3<f32> = (vec3<f32>(world_pos) - origin + 0.5 + vec3<f32>(sray) * 0.5) * inv_dir;
    var final_dist: f32 = 0.0;
    
    var dfv: u32 = 1u;
    var prev_world_pos: vec3<i32>;
    
    while final_dist < max_dist {
        let m: vec3<f32> = floor(f32(dfv - 1u) / delta);
        
        dist += m * dray;
        prev_world_pos = world_pos;
        world_pos += vec3<i32>(m) * sray;
        
        let mask: vec3<f32> = vec3(
            f32(dist.x <= dist.y && dist.x <= dist.z),
            f32(dist.y <= dist.x && dist.y <= dist.z),
            f32(dist.z <= dist.x && dist.z <= dist.y),
        );
        dist += mask * dray;
        prev_world_pos = world_pos;
        world_pos += vec3<i32>(mask) * sray;
        
        if any(world_pos < 0)
        || world_pos.x >= i32(WORLD_W)
        || world_pos.y >= i32(WORLD_H)
        || world_pos.z >= i32(WORLD_W) {
            return result;
        }
        
        final_dist = min(dist.x, min(dist.y, dist.z));
        
        // calculate the index of the voxel
        let byte_idx: u32 = 
            u32(world_pos.z) * WORLD_W * WORLD_H + 
            u32(world_pos.y) * WORLD_W + 
            u32(world_pos.x);
        
        let shift: u32 = (byte_idx % 4u) * 8u;
        let int_idx: i32 = i32(byte_idx / 4u);
        
        let voxel = (world_.voxels[int_idx] >> shift) & 0xFFu;
        
        // if the voxel is air, we can skip the voxel and more
        if voxel == 0u {
            dfv = (sdf_[int_idx] >> shift) & 0xFFu;
            continue;
        }
        
        // done
        result.norm = vec3<f32>(prev_world_pos - world_pos);
        result.color = voxel_colors__[voxel];
        result.hit = true;
        result.pos = origin + dir * final_dist;
        result.reflection = voxel_reflection__[voxel];
        result.color *= dot(result.norm, vec3(0.0, 1.0, 0.0)) * 0.5 + 1.0;
        return result;
    }
    return result;
}

fn cast_ray1(ray: Ray, max_dist: f32) -> HitResult {
    let dir = ray.dir;
    let start = ray.origin;
    var result: HitResult;
    result.color = settings_.sky_color;
    // -- DDA algorithm --

    // length of a line in same direction as the ray,
    // that travels 1 unit in the X, Y, Z
    let unit_step_size = vec3(
        sqrt(1.0 + (dir.y / dir.x) * (dir.y / dir.x) + (dir.z / dir.x) * (dir.z / dir.x)),
        sqrt(1.0 + (dir.x / dir.y) * (dir.x / dir.y) + (dir.z / dir.y) * (dir.z / dir.y)),
        sqrt(1.0 + (dir.x / dir.z) * (dir.x / dir.z) + (dir.y / dir.z) * (dir.y / dir.z)),
    );
    
    var world_pos: vec3<i32> = vec3(i32(start.x), i32(start.y), i32(start.z));
    
    // if the position of the voxel is outside the world, don't ray cast
    if world_pos.x < 0 || world_pos.y < 0 || world_pos.z < 0 
    || world_pos.x >= i32(WORLD_W)
    || world_pos.y >= i32(WORLD_H)
    || world_pos.z >= i32(WORLD_W) {
        return result;
    }
    
    var step: vec3<i32>;
    var ray_len: vec3<f32>;
    var world_edge: vec3<i32>;
    
    if dir.x < 0.0 {
        step.x = -1;
        world_edge.x = 0;
        ray_len.x = (start.x - f32(world_pos.x)) * unit_step_size.x;
    } else {
        step.x = 1;
        world_edge.x = i32(WORLD_W) - 1;
        ray_len.x = (f32(world_pos.x + 1) - start.x) * unit_step_size.x;
    }
    if dir.y < 0.0 {
        step.y = -1;
        world_edge.y = 0;
        ray_len.y = (start.y - f32(world_pos.y)) * unit_step_size.y;
    } else {
        step.y = 1;
        world_edge.y = i32(WORLD_H) - 1;
        ray_len.y = (f32(world_pos.y + 1) - start.y) * unit_step_size.y;
    }
    if dir.z < 0.0 {
        step.z = -1;
        world_edge.z = 0;
        ray_len.z = (start.z - f32(world_pos.z)) * unit_step_size.z;
    } else {
        step.z = 1;
        world_edge.z = i32(WORLD_W) - 1;
        ray_len.z = (f32(world_pos.z + 1) - start.z) * unit_step_size.z;
    }
    
    var dist: f32 = 0.0;
    var prev_world_pos = world_pos;
    var steps_todo: u32 = 1u;

    while dist < max_dist {
        while steps_todo > 0u {
            steps_todo -= 1u;
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
            
            if world_pos.x == world_edge.x
            || world_pos.y == world_edge.y
            || world_pos.z == world_edge.z {
                return result;
            }
        }
        
        // calculate the index of the voxel
        let byte_idx: u32 = 
            u32(world_pos.z) * WORLD_W * WORLD_H + 
            u32(world_pos.y) * WORLD_W + 
            u32(world_pos.x);
        
        let shift: u32 = (byte_idx % 4u) * 8u;
        let int_idx: i32 = i32(byte_idx / 4u);
        
        let voxel = (world_.voxels[int_idx] >> shift) & 0xFFu;
        steps_todo = (sdf_[int_idx] >> shift) & 0xFFu;
        
        // make sure the voxel is solid
        if voxel == 0u {
            continue;
        }
        
        // done
        result.norm = vec3<f32>(prev_world_pos - world_pos);
        result.color = voxel_colors__[voxel];
        result.hit = true;
        result.pos = start + dir * dist;
        result.reflection = voxel_reflection__[voxel];
        result.color *= dot(result.norm, vec3(0.0, 1.0, 0.0)) * 0.5 + 1.0;
        return result;
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

	var ray = create_ray_from_screen(screen_pos);
    var result: HitResult;
    if settings_.ray_cast_method == 0u {
        result = cast_ray1(ray, f32(settings_.max_ray_steps));
    }
    if settings_.ray_cast_method == 1u {
        result = cast_ray2(ray, f32(settings_.max_ray_steps));
    }
    var reflect_count: u32 = 0u;
    
    while result.reflection > 0.0 && reflect_count < settings_.max_reflections {
        reflect_count = reflect_count + 1u;
        
        let factor = result.reflection;
        let original_color = result.color;
        
        var new_ray: Ray;
        new_ray.dir = ray.dir - 2.0 * result.norm * dot(result.norm, ray.dir);
        new_ray.origin = result.pos + new_ray.dir * 0.01;
    
        result = cast_ray1(new_ray, f32(settings_.max_ray_steps));
        result.color = overlay_color(original_color, result.color, factor);
    }
    if result.hit && settings_.shadows == 1u {
        var to_sun: Ray;
        to_sun.dir = normalize(settings_.sun_pos - result.pos);
        to_sun.origin = result.pos + result.norm * 0.001;
        let to_sun_result = cast_ray1(to_sun, 50.0);
        if to_sun_result.hit {
            result.color *= 0.6;
        }
    }
    
    textureStore(color_buffer_, screen_pos, result.color);
}
