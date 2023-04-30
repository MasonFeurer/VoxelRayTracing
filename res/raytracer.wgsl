struct CamData {
    pos: vec3<f32>,
    inv_view_mat: mat4x4<f32>,
    inv_proj_mat: mat4x4<f32>,
    proj_size: vec2<f32>,
}

struct Settings {
    max_ray_steps: u32,
    sky_color: vec4<f32>,
    sun_pos: vec3<f32>,
    max_reflections: u32,
    shadows: u32, // bool
}

struct Node {
    data: u32,
    first_child: u32,
}
fn node_voxel(node: Node) -> u32 {
    return (node.data & 0xFF00u) >> 8u;
}
fn node_is_split(node: Node) -> bool {
    return bool(node.data & 1u);
}

struct World {
    root_idx: u32,
    size: u32,
    _max_depth: u32,
    _start_search: u32,
    nodes: array<Node>,
}

struct VoxelProps {
    color: vec3<f32>,
    pass_chance: f32,
    emission: f32,
    reflect_chance: f32,
}

@group(0) @binding(0) var output_texture_: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> cam_data_: CamData;
@group(0) @binding(2) var<uniform> settings_: Settings;
@group(0) @binding(3) var<storage, read> world_: World;
@group(0) @binding(4) var<storage, read> rand_src_: array<f32, 128>;
@group(0) @binding(5) var<storage, read> voxel_props_: array<VoxelProps>;

var<private> rand_float_idx__: i32 = 0;

const AIR: u32 = 0u;
const STONE: u32 = 1u;
const DIRT: u32 = 2u;
const GRASS: u32 = 3u;
const FIRE: u32 = 4u;
const MAGMA: u32 = 5u;
const WATER: u32 = 6u;
const SAND: u32 = 10u;

fn rand_float() -> f32 {
    let f = rand_src_[rand_float_idx__];
    rand_float_idx__ += 1;
    if rand_float_idx__ >= 128 {
        rand_float_idx__ = 0;
    }
    return f;
}

fn rng_next(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;
    var result = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    result = (result >> 22u) ^ result;
    return f32(result) / 4294967295.0;
}
fn rng_next_norm(state: ptr<function, u32>) -> f32 {
    let theta = 2.0 * 3.14159265 * rng_next(state);
    let rho = sqrt(-2.0 * log(rng_next(state)));
    return rho * cos(theta);
}
fn rng_next_dir(state: ptr<function, u32>) -> vec3<f32> {
    let x = rng_next_norm(state);
    let y = rng_next_norm(state);
    let z = rng_next_norm(state);
    return normalize(vec3(x, y, z));
}
fn rng_next_hem_dir(state: ptr<function, u32>, norm: vec3<f32>) -> vec3<f32> {
    let dir = rng_next_dir(state);
    return dir * sign(dot(norm, dir));
}

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
}
struct HitResult {
	hit: bool,
    color: vec3<f32>,
    norm: vec3<f32>,
    pos: vec3<f32>,
    reflect_chance: f32,
}

fn get_node(idx: u32) -> Node {
    return world_.nodes[idx];
}
fn get_node_child(idx: u32, child: u32) -> u32 {
    return world_.nodes[idx].first_child + child;
}

struct FoundNode {
    idx: u32,
    min: vec3<f32>,
    max: vec3<f32>,
}

fn find_node(pos: vec3<f32>) -> FoundNode {
    var size = f32(world_.size);
    var center = vec3(size * 0.5);
    var node_idx = world_.root_idx;
    
    var iter_count: u32 = 0u;
    while iter_count < 100u {
        iter_count += 1u;
        
        let node = get_node(node_idx);
        if !node_is_split(node) {
            var out: FoundNode;
            out.idx = node_idx;
            out.min = vec3<f32>(center) - vec3(size * 0.5);
            out.max = vec3<f32>(center) + vec3(size * 0.5);
            return out;
        }
        size *= 0.5;
        
        let gt: vec3<bool> = pos >= center;
        let child_idx = 
            u32(gt.x) << 0u |
            u32(gt.y) << 1u |
            u32(gt.z) << 2u;
        
        node_idx = get_node_child(node_idx, child_idx);
        let child_dir = vec3<i32>(gt) * 2 - vec3(1);
        center += (size * 0.5) * vec3<f32>(child_dir);
    }
    // this shouldn't happen, even if `pos` is outside of the world bounds
    var out: FoundNode;
    return out;
}

fn cast_ray(rng: ptr<function, u32>, start_ray: Ray) -> HitResult {
    let dir = start_ray.dir;
    let mask = vec3<f32>(dir > 0.0);
    let imask = 1.0 - mask;
    
    var ray_pos = start_ray.origin;
    
    let world_min = vec3(0.0);
    let world_max = vec3(f32(world_.size));
    
    var result: HitResult;
    result.color = settings_.sky_color.xyz;
    
    if any(ray_pos <= world_min) | any(ray_pos >= world_max) {
        return result;
    }
    
    // length of a line in same direction as the ray,
    // that travels 1 unit in the X, Y, Z
    let unit_step_size = vec3(
        sqrt(1.0 + (dir.y / dir.x) * (dir.y / dir.x) + (dir.z / dir.x) * (dir.z / dir.x)),
        sqrt(1.0 + (dir.x / dir.y) * (dir.x / dir.y) + (dir.z / dir.y) * (dir.z / dir.y)),
        sqrt(1.0 + (dir.x / dir.z) * (dir.x / dir.z) + (dir.y / dir.z) * (dir.y / dir.z)),
    );
    
    var voxel: u32;
    var norm: vec3<f32>;
    
    var iter_count: u32 = 0u;
    while iter_count < 100u {
        iter_count += 1u;
        
        let found_node = find_node(ray_pos);
        voxel = node_voxel(get_node(found_node.idx));
        
        if voxel_props_[voxel].pass_chance != 1.0 {
            break;
        }
        let node_min = vec3<f32>(found_node.min);
        let node_max = vec3<f32>(found_node.max);
        
        let axis_dist = ((ray_pos - node_min) * imask + (node_max - ray_pos) * mask) * unit_step_size;
        let step = min(axis_dist.x, min(axis_dist.y, axis_dist.z));
        
        norm = vec3<f32>(step == axis_dist);
        ray_pos += dir * (step + 0.001);
        
        if any(ray_pos < world_min) | any(ray_pos >= world_max) {
            return result;
        }
    }
    
    result.norm = norm;
    result.color = voxel_props_[voxel].color;
    result.hit = true;
    result.pos = ray_pos;
    result.reflect_chance = voxel_props_[voxel].reflect_chance;
    
    if result.norm.x != 0.0 {
        result.color *= 0.5;
    }
    if result.norm.z != 0.0 {
        result.color *= 0.7;
    }
    if result.norm.y == -1.0 {
        result.color *= 0.2;
    }
    return result;
}

fn create_ray_from_screen(screen_pos: vec2<i32>) -> Ray {
	let x = (f32(screen_pos.x) * 2.0) / cam_data_.proj_size.x - 1.0;
	let y = (f32(screen_pos.y) * 2.0) / cam_data_.proj_size.y - 1.0;
	let clip_coords = vec4(x, -y, -1.0, 1.0);
	let eye_coords0 = clip_coords * cam_data_.inv_proj_mat;
	let eye_coords = vec4(eye_coords0.xy, -1.0, 0.0);
	let ray_world = normalize((eye_coords * cam_data_.inv_view_mat).xyz);
	
	var ray: Ray;
	ray.origin = cam_data_.pos;
	ray.dir = ray_world;
	return ray;
}

fn overlay_color(back: vec4<f32>, front: vec4<f32>, factor: f32) -> vec4<f32> {
    return back * (1.0 - factor) + front * factor;
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) inv_id: vec3<u32>) {
    let screen_pos = vec2<i32>(inv_id.xy);
    var rng = inv_id.y * u32(cam_data_.proj_size.x) + inv_id.x;

	var ray = create_ray_from_screen(screen_pos);
    var result = cast_ray(&rng, ray);
    var reflect_count: u32 = 0u;

//    while reflect_count < settings_.max_reflections {
//        reflect_count += 1u;
//        
//        let original_color = vec4(result.color, 1.0);
//        
//        var specular_ray: Ray;
//        specular_ray.dir = ray.dir - 2.0 * result.norm * dot(result.norm, ray.dir);
//        specular_ray.origin = result.pos + specular_ray.dir * 0.01;
//        
//        var scattered_ray: Ray;
//        scattered_ray.dir = rng_next_hem_dir(&rng, result.norm);
//        scattered_ray.origin = result.pos + scattered_ray.dir * 0.01;
//        
//        result = cast_ray(&rng, specular_ray);
//    
//        result.color = overlay_color(original_color, vec4(result.color, 1.0), 0.5).xyz;
//        
//        break;
//        // if rng_next(&rng) > result.reflect_chance {
//        //     break;
//        // }
//    }
    if result.hit && settings_.shadows == 1u {
        var to_sun: Ray;
        to_sun.dir = normalize(settings_.sun_pos - result.pos);
        to_sun.origin = result.pos + result.norm * 0.001;
        let to_sun_result = cast_ray(&rng, to_sun);
        if to_sun_result.hit {
            result.color *= 0.6;
        }
    }
    result.color = result.color;
    
    textureStore(output_texture_, screen_pos, vec4(result.color, 1.0));
}

//typedef struct Pos2 {
//    float x;
//    float y;
//} Pos2;
//
//Pos2* rayBoxIntersection(Pos2 rayPos, Pos2 rayDir, Pos2 boxMin, Pos2 boxMax) {
//    float tmin = (boxMin.x - rayPos.x) / rayDir.x;
//    float tmax = (boxMax.x - rayPos.x) / rayDir.x;
//
//    if (tmin > tmax) {
//        float temp = tmin;
//        tmin = tmax;
//        tmax = temp;
//    }
//
//    float tymin = (boxMin.y - rayPos.y) / rayDir.y;
//    float tymax = (boxMax.y - rayPos.y) / rayDir.y;
//
//    if (tymin > tymax) {
//        float temp = tymin;
//        tymin = tymax;
//        tymax = temp;
//    }
//
//    if ((tmin > tymax) || (tymin > tmax)) {
//        return NULL;
//    }
//
//    if (tymin > tmin) {
//        tmin = tymin;
//    }
//
//    if (tymax < tmax) {
//        tmax = tymax;
//    }
//
//    Pos2* intersection = malloc(sizeof(Pos2));
//    intersection->x = rayPos.x + (rayDir.x * tmin);
//    intersection->y = rayPos.y + (rayDir.y * tmin);
//
//    return intersection;
//}
