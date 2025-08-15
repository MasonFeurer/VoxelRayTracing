struct CamData {
    pos: vec3<f32>,
    inv_view_mat: mat4x4<f32>,
    inv_proj_mat: mat4x4<f32>,
    proj_size: vec2<f32>,
}

struct Settings {
    max_ray_bounces: u32,
    sun_intensity: f32,
    show_step_count: u32,
    sky_color: vec3<f32>,
    sun_pos: vec3<f32>,
    samples_per_pixel: u32,
}

struct World {
    min: vec3<i32>,
    size: u32,
    size_in_chunks: u32,
}

struct Material {
    color: vec3<f32>,
    empty: u32,
    scatter: f32,
    emission: f32,
    polish_bounce_chance: f32,
    translucency: f32,
    polish_color: vec3<f32>,
    polish_scatter: f32,
}

struct ChunkHeader {
    root: u32,
    alloc: u32,
}

@group(0) @binding(0) var output_texture_: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> cam_data_: CamData;
@group(0) @binding(2) var<uniform> settings_: Settings;
@group(0) @binding(3) var<storage, read> voxel_mats: array<Material>;

@group(0) @binding(5) var<uniform> world_: World;
@group(0) @binding(6) var<storage, read> nodes_: array<u32>;
@group(0) @binding(7) var<storage, read> chunks_: array<ChunkHeader>;

fn get_node(idx: u32) -> u32 {
    return nodes_[idx];
}
fn node_is_split(node: u32) -> bool {
    return bool(node >> 31u); // only MSB
}
fn node_voxel(node: u32) -> u32 {
    return node & 0x7FFFFFFFu; // all except MSB
}
fn node_child_idx(node: u32) -> u32 {
    return node & 0x7FFFFFFFu; // all except MSB
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

struct FoundNode {
    idx: u32,
    min: vec3<f32>,
    max: vec3<f32>,
    center: vec3<f32>,
    size: f32,
}

fn find_chunk_node(
    pos: vec3<f32>,
    max_depth: u32,
    min: vec3<f32>,
    root: u32,
) -> FoundNode {
    var center = min + vec3(16.0);
    var size = 32.0;
    var idx = root;
    var depth: u32 = 0u;

    loop {
        let node = get_node(idx);
        if !node_is_split(node) || depth == max_depth {
            var out: FoundNode;
            out.idx = idx;
            out.min = center - vec3(size * 0.5);
            out.max = center + vec3(size * 0.5);
            out.center = center;
            out.size = size;
            return out;
        }
        size *= 0.5;

        let gt = vec3(
            i32(pos.x >= center.x),
            i32(pos.y >= center.y),
            i32(pos.z >= center.z),
        );
        let child_idx = u32(gt.x) << 0u | u32(gt.y) << 1u | u32(gt.z) << 2u;
        idx = node_child_idx(node) + child_idx;
        let child_dir = gt * 2 - vec3(1);
        center += vec3(size * 0.5) * vec3<f32>(child_dir);
        depth += 1u;
    }
    var out: FoundNode;
    return out;
}

fn find_node(pos: vec3<f32>, max_depth: u32) -> FoundNode {
    let world_chunk_w = world_.size_in_chunks;
    let chunk_coords = vec3<i32>(floor(pos / 32.0));
    let min = vec3<f32>(chunk_coords * 32);
    let chunk_idx = u32(chunk_coords.x)
        + u32(chunk_coords.y) * world_chunk_w
        + u32(chunk_coords.z) * world_chunk_w * world_chunk_w;
    let root = chunks_[chunk_idx].root;
    return find_chunk_node(pos, max_depth, min, root);
}

struct HitResult {
    hit: bool,
    material: Material,
    norm: vec3<f32>,
    pos: vec3<f32>,
    water_dist: f32,
}

fn ray_color(rng: ptr<function, u32>, ray_in: Ray) -> vec3<f32> {
	var ray = ray_in;
    var ray_color: vec3<f32> = vec3(1.0);
    var incoming_light: vec3<f32> = vec3(0.0);
    
    var bounce_count = 0u;
    while bounce_count < settings_.max_ray_bounces {
        let rs: HitResult = ray_world(ray);
        if !rs.hit {
            let color = ray_sky(ray);
            incoming_light += color * ray_color;
            break;
        }

        if rs.hit {

        }
        
        // if rng_next(rng) < rs.material.translucency {
        // if rs.material.translucency != 0.0 {
        //     ray.origin = rs.pos + ray.dir * 0.001;
        //     // ray_color *= (rs.material.color * 0.1);
        //     bounce_count += 1u;
        //     continue;
        // }

        // let is_polish_bounce = rng_next(rng) < rs.material.polish_bounce_chance;
        
        let specular_dir = ray.dir - 2.0 * rs.norm * dot(rs.norm, ray.dir);
        let scattered_dir = normalize(rs.norm + rng_next_dir(rng));
        
        // let scatter = mix(rs.material.scatter, rs.material.polish_scatter, f32(is_polish_bounce));
        let scatter = rs.material.scatter;
        
        let emitted_light = rs.material.color * rs.material.emission;
        incoming_light += emitted_light * ray_color;
        // ray_color *= mix(rs.material.color, rs.material.polish_color, f32(is_polish_bounce));
        ray_color *= rs.material.color;

        ray.dir = normalize(mix(specular_dir, scattered_dir, scatter));
        ray.origin = rs.pos + ray.dir * 0.001;
        
        bounce_count += 1u;
    }
    return incoming_light;
}

fn ray_world(start_ray: Ray) -> HitResult {
    let dir = start_ray.dir;
    let mask: vec3<f32> = vec3(f32(dir.x >= 0.0), f32(dir.y >= 0.0), f32(dir.z >= 0.0));
    let imask: vec3<f32> = 1.0 - mask;
    
    var ray_pos = start_ray.origin;
    
    let world_min = vec3(0.0);
    let world_max = world_min + f32(world_.size);
    
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
    while iter_count < 200u {
        iter_count += 1u;
        
        let found_node = find_node(ray_pos, 5u); // the most child one
        voxel = node_voxel(get_node(found_node.idx)); // just voxel - most time air
        
        if voxel != 0u { // not air, so return it
            break;
        }
        let axis_dist: vec3<f32> = (
            (ray_pos - found_node.min) * imask + (found_node.max - ray_pos) * mask
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

        norm = vec3<f32>(f32(step == axis_dist.x), f32(step == axis_dist.y), f32(step == axis_dist.z)) * -sign(dir);
        ray_pos += dir * (step + 0.001) * vec3<f32>(f32(step == axis_dist.x), f32(step == axis_dist.y), f32(step == axis_dist.z)) + 
            dir * (step) * vec3<f32>(f32(step != axis_dist.x), f32(step != axis_dist.y), f32(step != axis_dist.z));
        
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

    if settings_.show_step_count == 1u {
        let f = f32(iter_count) / 200.0;
        result.material.color = vec3(clamp(f, 0.0, 1.0));
    }
    return result;
}

fn ray_sky(ray: Ray) -> vec3<f32> {
    let horizon_color = vec3(1.0, 0.3, 0.0);
    let void_color = vec3(0.03);
    let sun_size = 0.01;
    
    let ground_to_sky_t = smoothstep(-0.01, 0.0, ray.dir.y);
    let sky_gradient_t = pow(smoothstep(0.0, 0.4, ray.dir.y), 0.35);
    let sky_gradient = mix(horizon_color, settings_.sky_color, sky_gradient_t);
    let sun_dir = normalize(settings_.sun_pos - vec3<f32>(world_.min) - ray.origin);
    
    let sun = f32(dot(ray.dir, sun_dir) > (1.0 - sun_size) && ground_to_sky_t >= 1.0);
    
    return mix(void_color, sky_gradient, ground_to_sky_t) + sun * settings_.sun_intensity;
}

fn create_ray_from_screen(screen_pos: vec2<i32>) -> Ray {
    let x = (f32(screen_pos.x) * 2.0) / cam_data_.proj_size.x - 1.0;
    let y = (f32(screen_pos.y) * 2.0) / cam_data_.proj_size.y - 1.0;
    let clip_coords = vec4(x, -y, -1.0, 1.0);
    let eye_coords0 = clip_coords * cam_data_.inv_proj_mat;
    let eye_coords = vec4(eye_coords0.xy, -1.0, 0.0);
    let ray_world = normalize((eye_coords * cam_data_.inv_view_mat).xyz);

    var ray: Ray;
    ray.origin = cam_data_.pos - vec3<f32>(world_.min);
    ray.dir = ray_world;
    return ray;
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) inv_id: vec3<u32>) {
    let screen_pos = vec2<i32>(inv_id.xy);
    // var rng = inv_id.y * u32(cam_data_.proj_size.x) + inv_id.x + frame_count_ * 27927421u;
    var rng = inv_id.y * u32(cam_data_.proj_size.x) + inv_id.x;
    
    let ray = create_ray_from_screen(screen_pos);
    let color = ray_color(&rng, ray);
    
    textureStore(output_texture_, screen_pos, vec4(color, 1.0));
}
