struct CamData {
    pos: vec3<f32>,
    inv_view_mat: mat4x4<f32>,
    inv_proj_mat: mat4x4<f32>,
    proj_size: vec2<f32>,
}

struct Settings {
    samples_per_pixel: u32,
    max_ray_bounces: u32,
    sun_intensity: f32,
    sky_color: vec3<f32>,
    sun_pos: vec3<f32>,
}

struct Node {
    data: u32,
}
fn get_bits(field: u32, len: u32, offset: u32) -> u32 {
    let mask = !(!0u << len) << offset;
    return (field & mask) >> offset;
}

fn node_voxel(node: Node) -> u32 {
    return get_bits(node.data, 8u, 0u);
}
fn node_is_split(node: Node) -> bool {
    return get_bits(node.data, 1u, 31u) == 1u;
}
fn node_child(node: Node, child: u32) -> u32 {
    return get_bits(node.data, 30u, 0u) * 8u + 1u + child;
}

struct World {
    size: u32,
    _max_depth: u32,
    _start_search: u32,
    nodes: array<Node>,
}

struct Material {
    color: vec3<f32>,
    empty: u32,
    scatter: f32,
    emission: f32,
    polish_bounce_chance: f32,
    polish_color: vec3<f32>,
    polish_scatter: f32,
}

@group(0) @binding(0) var output_texture_: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> cam_data_: CamData;
@group(0) @binding(2) var<uniform> settings_: Settings;
@group(0) @binding(3) var<storage, read> world_: World;
@group(0) @binding(4) var<storage, read> voxel_mats: array<Material>;
@group(0) @binding(5) var<uniform> frame_count_: u32;

const AIR: u32 = 0u;
const STONE: u32 = 1u;
const DIRT: u32 = 2u;
const GRASS: u32 = 3u;
const FIRE: u32 = 4u;
const MAGMA: u32 = 5u;
const WATER: u32 = 6u;
const SAND: u32 = 10u;

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
    material: Material,
    norm: vec3<f32>,
    pos: vec3<f32>,
    debug_1: bool,
    debug_2: bool,
    debug_3: bool,
}

fn get_node(idx: u32) -> Node {
    return world_.nodes[idx];
}
fn get_node_child(idx: u32, child: u32) -> u32 {
    return node_child(world_.nodes[idx], child);
}

struct FoundNode {
    idx: u32,
    min: vec3<f32>,
    max: vec3<f32>,
    center: vec3<f32>,
    size: f32,
}

fn find_node(pos: vec3<f32>) -> FoundNode {
    var size = f32(world_.size);
    var center = vec3(size * 0.5);
    var node_idx = 0u;
    loop {
        let node = get_node(node_idx);
        if !node_is_split(node) {
            var out: FoundNode;
            out.idx = node_idx;
            out.min = vec3<f32>(center) - vec3(size * 0.5);
            out.max = vec3<f32>(center) + vec3(size * 0.5);
            out.center = vec3<f32>(center);
            out.size = size;
            return out;
        }
        size *= 0.5;

        // let gt: vec3<bool> = pos >= center;
        // let gt: vec3<bool> = pos - center >= 0.000000000000001;
        let gt: vec3<bool> = pos >= center;
        let child_idx = 
            u32(gt.x) << 0u |
            u32(gt.y) << 1u |
            u32(gt.z) << 2u;
        
        node_idx = get_node_child(node_idx, child_idx);
        let child_dir = vec3<f32>(gt) * 2.0 - vec3(1.0);
        center += (size * 0.5) * child_dir;
    }
    // this shouldn't happen, even if `pos` is outside of the world bounds
    var out: FoundNode;
    return out;
}

fn ray_color(rng: ptr<function, u32>, ray: Ray) -> vec3<f32> {
    let rs = ray_world(rng, ray);
    let sky_color = ray_sky(ray);
    var vox_color = rs.material.color;
    
    if all(vox_color == 0.0) {
        vox_color = vec3(1.0, 0.0, 0.0);
        if (rs.debug_3) { 
            vox_color = vec3(1.0, 0.0, 1.0);
        }
    }
    
    return vox_color * f32(rs.hit) + sky_color * f32(!rs.hit);
}

fn ray_sky(ray: Ray) -> vec3<f32> {
    let horizon_color = vec3(1.0, 0.3, 0.0);
    let void_color = vec3(0.03);
    let sun_size = 0.01;
    
    let ground_to_sky_t = smoothstep(-0.01, 0.0, ray.dir.y);
    let sky_gradient_t = pow(smoothstep(0.0, 0.4, ray.dir.y), 0.35);
    let sky_gradient = mix(horizon_color, settings_.sky_color, sky_gradient_t);
    let sun_dir = normalize(settings_.sun_pos - ray.origin);
    
    let sun = f32(dot(ray.dir, sun_dir) > (1.0 - sun_size) && ground_to_sky_t >= 1.0);
    
    return mix(void_color, sky_gradient, ground_to_sky_t) + sun * settings_.sun_intensity;
}

fn ray_world(rng: ptr<function, u32>, start_ray: Ray) -> HitResult {
    let dir = start_ray.dir;
    let mask = vec3<f32>(dir >= 0.0);
    let imask = 1.0 - mask;
    
    var ray_pos = start_ray.origin;
    
    let world_min = vec3(0.0);
    let world_max = vec3(f32(world_.size));
    
    var result: HitResult;
    
    if any(ray_pos <= world_min) | any(ray_pos >= world_max) {
        return result;
    }
    
    // length of a line in same direction as the ray,
    // that travels 1 unit in the X, Y, Z

    // dir - normilized --- x^2 + y^2 + z^2 = 1
    let unit_step_size = vec3(
        sqrt(1.0 + (dir.y / dir.x) * (dir.y / dir.x) + (dir.z / dir.x) * (dir.z / dir.x)),
        sqrt(1.0 + (dir.x / dir.y) * (dir.x / dir.y) + (dir.z / dir.y) * (dir.z / dir.y)),
        sqrt(1.0 + (dir.x / dir.z) * (dir.x / dir.z) + (dir.y / dir.z) * (dir.y / dir.z)),
    );
    
    var voxel: u32;
    var norm: vec3<f32>;
    
    var iter_count: u32 = 0u;
    while iter_count < 500u {
        iter_count += 1u;
        
        let found_node = find_node(ray_pos); // the most child one

        voxel = node_voxel(get_node(found_node.idx)); // just voxel - most time air
        
        if voxel_mats[voxel].empty == 0u { // not air, so return it
            break;
        }
        let node_min = vec3<f32>(found_node.min);
        let node_max = vec3<f32>(found_node.max);
        
        let axis_dist = (
            (ray_pos - node_min) * imask + (node_max - ray_pos) * mask
        ) * unit_step_size;

        var step: f32;

        if axis_dist.x == 0.0 {
            if axis_dist.y == 0.0 {
                step = axis_dist.z;
            } else if axis_dist.z == 0.0 {
                step = axis_dist.y;
            } else {
                step = min(axis_dist.y, axis_dist.z);
            }
        } else {
            if axis_dist.y == 0.0 {
                if axis_dist.z == 0.0 {
                    step = axis_dist.x;
                } else {
                    step = min(axis_dist.x, axis_dist.z);
                }
            } else {
                if axis_dist.z == 0.0 {
                    step = min(axis_dist.y, axis_dist.x);
                } else {
                    step = min(axis_dist.x, min(axis_dist.y, axis_dist.z));
                }
            }
        }

        norm = vec3<f32>(step == axis_dist) * -sign(dir);
        ray_pos += dir * (step + 0.001);
        
        if any(ray_pos < world_min) | any(ray_pos >= world_max) {
            return result;
        } // out of bounds
    } // return not air OR max steps already !!!!!!!!!!!

    result.hit = true;
    result.pos = ray_pos;
    result.norm = norm;
    result.material = voxel_mats[voxel];
    if result.norm.x != 0.0 {
        result.material.color *= 0.5;
    }
    if result.norm.z != 0.0 {
        result.material.color *= 0.7;
    }
    if result.norm.y == -1.0 {
        result.material.color *= 0.2;
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

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) inv_id: vec3<u32>) {
    let screen_pos = vec2<i32>(inv_id.xy);
    var rng = inv_id.y * u32(cam_data_.proj_size.x) + inv_id.x + frame_count_ * 27927421u;
    
    let ray = create_ray_from_screen(screen_pos);
    let color = ray_color(&rng, ray);
    
    textureStore(output_texture_, screen_pos, vec4(color, 1.0));
}
