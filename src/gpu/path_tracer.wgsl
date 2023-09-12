struct CamData {
    pos: vec3<f32>,
    inv_view_mat: mat4x4<f32>,
    inv_proj_mat: mat4x4<f32>,
    proj_size: vec2<f32>,
}

struct Settings {
    max_ray_bounces: u32,
    sun_intensity: f32,
    sky_color: vec3<f32>,
    sun_pos: vec3<f32>,
}

struct World {
    min: vec3<f32>,
    size: f32,
}

fn get_bits(field: u32, len: u32, offset: u32) -> u32 {
    let mask = !(!0u << len) << offset;
    return (field & mask) >> offset;
}

fn node_voxel(node_idx: u32) -> u32 {
    return get_bits(nodes_[node_idx], 8u, 0u);
}
fn node_is_split(node_idx: u32) -> bool {
    return get_bits(nodes_[node_idx], 1u, 31u) == 1u;
}
fn node_child(node_idx: u32, child: u32) -> u32 {
    return get_bits(nodes_[node_idx], 30u, 0u) * 8u + 1u + child;
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
@group(0) @binding(3) var<storage, read> nodes_: array<u32>;
@group(0) @binding(4) var<storage, read> voxel_mats: array<Material>;
@group(0) @binding(5) var<uniform> frame_count_: u32;
@group(0) @binding(6) var<uniform> world_: World;

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
    var center = world_.min + vec3(size * 0.5);
    var node_idx = 0u;
    
    loop {
        if !node_is_split(node_idx) {
            var out: FoundNode;
            out.idx = node_idx;
            out.min = vec3<f32>(center) - vec3(size * 0.5);
            out.max = vec3<f32>(center) + vec3(size * 0.5);
            out.center = vec3<f32>(center);
            out.size = size;
            return out;
        }
        size *= 0.5;
        
        let gt: vec3<bool> = pos >= center;
        let child_idx = 
            u32(gt.x) << 0u |
            u32(gt.y) << 1u |
            u32(gt.z) << 2u;
        
        node_idx = node_child(node_idx, child_idx);
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
        
        let is_polish_bounce = rng_next(rng) <= rs.material.polish_bounce_chance;
        
        let specular_dir = ray.dir - 2.0 * rs.norm * dot(rs.norm, ray.dir);
        let scattered_dir = normalize(rs.norm + rng_next_dir(rng));
        
        let scatter = mix(rs.material.scatter, rs.material.polish_scatter, f32(is_polish_bounce));
        
        ray.dir = normalize(mix(specular_dir, scattered_dir, scatter));
        ray.origin = rs.pos + ray.dir * 0.001;
        
        incoming_light += (rs.material.color * rs.material.emission) * ray_color;
        ray_color *= mix(rs.material.color, rs.material.polish_color, f32(is_polish_bounce));
        
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
    
    return mix(void_color, sky_gradient, ground_to_sky_t) + sun * settings_.sun_intensity;
}

fn ray_world(rng: ptr<function, u32>, start_ray: Ray) -> HitResult {
    let dir = start_ray.dir;
    let mask = vec3<f32>(dir >= 0.0);
    let imask = 1.0 - mask;
    
    var ray_pos = start_ray.origin;
    
    let world_min = world_.min;
    let world_max = world_min + vec3(f32(world_.size));
    
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
        voxel = node_voxel(found_node.idx); // just voxel - most time air
        
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
        ray_pos += dir * (step + 0.001) * vec3<f32>(step == axis_dist) + 
            dir * (step) * vec3<f32>(step != axis_dist);
        
        if any(ray_pos < world_min) | any(ray_pos >= world_max) {
            return result;
        } // out of bounds
    } // return not air OR max steps already !!!!!!!!!!!
    
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

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) inv_id: vec3<u32>) {
    let screen_pos = vec2<i32>(inv_id.xy);
    var rng = inv_id.y * u32(cam_data_.proj_size.x) + inv_id.x + frame_count_ * 27927421u;

    let ray = create_ray_from_screen(screen_pos);
    let color = ray_color(&rng, ray);
    textureStore(output_texture_, screen_pos, vec4(color, 1.0));
}
