struct CamData {
    pos: vec3<f32>,
    inv_view_mat: mat4x4<f32>,
    inv_proj_mat: mat4x4<f32>,
    proj_size: vec2<f32>,
}

struct Settings {
    samples_per_pixel: u32,
    max_ray_steps: u32,
    max_ray_bounces: u32,
    sky_color: vec3<f32>,
    sun_pos: vec3<f32>,
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

struct Material {
    color: vec3<f32>,
    empty: u32,
    scatter: f32,
    emission: f32,
    specular_chance: f32,
}

@group(0) @binding(0) var output_texture_: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(6) var prev_output_texture_: texture_2d<f32>;
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

// fn guassian_weight(x: vec3<f32>, y: vec3<f32>, sigma: f32) -> f32 {
//     let dist_sq = dot(x - y, x - y);
//     return exp(-dist_sq / (2.0 * sigma * sigma));
// }

// fn bilateral_filter(center_color: vec3<f32>, center_coords: vec3<i32>) -> vec4<f32> {
//     var result = vec3(0.0);
//     var weight_sum = 0.0;
//     
//     var i: i32 = -KERNAL_SIZE;
//     while i <= KERNAL_SIZE {
//         var j: i32 = -KERNAL_SIZE;
//         while j <= KERNAL_SIZE {
//             let current_coords: vec2<i32> = center_coords + vec2(i, j);
//             let current_color: vec3<f32> = texelFetch(inputTexture, current_coords, 0).rgb;
//             
//             let color_weight: f32 = guassian_weight(center_color, current_color, SIGMA_COLOR);
//             let spatial_weight: f32 = guassian_weight(vec3(center_coords), vec3(current_coords), SIGMA_SPACE);
//             let weight: f32 = color_weight * spatial_weight;
//             
//             result += current_color * weight;
//             weight_sum += weight;
//             
//             j += 1;
//         }
//         i += 1;
//     }
//     
//     return vec4(result / weight_sum, 1.0);
// }

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

struct HitResult2 {
    hit: bool,
    material: Material,
    norm: vec3<f32>,
    pos: vec3<f32>,
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
    
    loop {
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
        let child_dir = vec3<f32>(gt) * 2.0 - vec3(1.0);
        center += (size * 0.5) * child_dir;
    }
    // this shouldn't happen, even if `pos` is outside of the world bounds
    var out: FoundNode;
    return out;
}

fn ray_color(rng: ptr<function, u32>, ray: Ray) -> vec3<f32> {
    var ray = ray;
    var ray_color: vec3<f32> = vec3(1.0);
    var incoming_light: vec3<f32> = vec3(0.0);
    
    var bounce_count = 0u;
    while bounce_count < settings_.max_ray_bounces {
        let rs = ray_world(rng, ray);
        if !rs.hit {
            let color = ray_sky(ray);
            incoming_light += color * ray_color;
            break;
        }
        
        // var specular_ray: Ray;
        // specular_ray.dir = ray.dir - 2.0 * rs.norm * dot(rs.norm, ray.dir);
        // specular_ray.origin = rs.pos + specular_ray.dir * 0.001;
        
        var scattered_ray: Ray;
        scattered_ray.dir = normalize(rs.norm + rng_next_dir(rng));
        // scattered_ray.dir = rng_next_hem_dir(rng, rs.norm);
        scattered_ray.origin = rs.pos + scattered_ray.dir * 0.001;
        
        ray = scattered_ray;
        incoming_light += (rs.material.color * rs.material.emission) * ray_color;
        ray_color *= rs.material.color;
        
        bounce_count += 1u;
    }
    return incoming_light;
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
    
    return mix(void_color, sky_gradient, ground_to_sky_t) + sun * 3.0;
}

fn ray_world(rng: ptr<function, u32>, start_ray: Ray) -> HitResult2 {
    let dir = start_ray.dir;
    // TODO: move `mask`, `imask`, and `unit_step_size` to `update`.
    //       It only needs to be calculated per pixel.
    let mask = vec3<f32>(dir > 0.0);
    let imask = 1.0 - mask;
    
    var ray_pos = start_ray.origin;
    
    let world_min = vec3(0.0);
    let world_max = vec3(f32(world_.size));
    
    var result: HitResult2;
    
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
    while iter_count < 50u {
        iter_count += 1u;
        
        let found_node = find_node(ray_pos);
        voxel = node_voxel(get_node(found_node.idx));
        
        if voxel_mats[voxel].empty == 0u {
            break;
        }
        let node_min = vec3<f32>(found_node.min);
        let node_max = vec3<f32>(found_node.max);
        
        let axis_dist = ((ray_pos - node_min) * imask + (node_max - ray_pos) * mask) * unit_step_size;
        let step = min(axis_dist.x, min(axis_dist.y, axis_dist.z));
        
        norm = vec3<f32>(step == axis_dist) * -sign(dir);
        ray_pos += dir * (step + 0.001);
        
        if any(ray_pos < world_min) | any(ray_pos >= world_max) {
            return result;
        }
    }
    
    result.hit = true;
    result.pos = ray_pos;
    result.norm = norm;
    result.material = voxel_mats[voxel];
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
    var rng = inv_id.y * u32(cam_data_.proj_size.x) + inv_id.x + frame_count_ * 27927421u;

    let ray = create_ray_from_screen(screen_pos);
    
    var color = vec3(0.0);
    var ray_count = 0u;
    while ray_count < settings_.samples_per_pixel {
        color += ray_color(&rng, ray);
        ray_count += 1u;
    }
    color /= f32(ray_count);
    
    let old_render = textureLoad(prev_output_texture_, screen_pos, 0);
    let weight = 1.0 / f32(frame_count_ + 1u);
    let result = old_render * (1.0 - weight) + vec4(color, 1.0) * weight;
    // let result = vec4(color, 1.0);
    
    textureStore(output_texture_, screen_pos, result);
}
