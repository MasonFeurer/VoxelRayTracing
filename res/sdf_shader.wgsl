const WORLD_W: u32 = 256u;
const WORLD_H: u32 = 256u;
const WORLD_VOLUME: u32 = 16777216u; // WORLD_W * WORLD_H * WORLD_W
const WORLD_INT_COUNT: u32 = 4194304u; // WORLD_VOLUME / 4

struct World {
    origin: vec3<u32>,
    updated: u32, // bool
    voxels: array<u32, WORLD_INT_COUNT>,
}

@group(0) @binding(0) var<storage, read_write> sdf_: array<u32, WORLD_INT_COUNT>;
@group(0) @binding(1) var<storage, read> world_: World;
@group(0) @binding(2) var<uniform> max_dist_: u32;

fn get_voxel(pos: vec3<i32>) -> u32 {
	if pos.x < 0
	|| pos.y < 0
	|| pos.z < 0
	|| pos.x >= i32(WORLD_W)
	|| pos.y >= i32(WORLD_H)
	|| pos.z >= i32(WORLD_W) {
		return 256u;
	}
	
	let byte_idx: u32 = 
            u32(pos.z) * WORLD_W * WORLD_H + 
            u32(pos.y) * WORLD_W + 
            u32(pos.x);
	
    return (world_.voxels[i32(byte_idx / 4u)] >> (byte_idx % 4u) * 8u) & 0xFFu;
}

// 230 total world voxel checks at max level (6)
fn compute_voxel_dfv(pos: vec3<i32>) -> u32 {
	// DFV = Distance Field Value (the value for this voxel in the SDF)
	// start off by setting the voxel's DFV to 1
	var dfv: u32 = 1u;
    
    // if the voxel is solid, we will not create a DFV (distance field value)
	// if voxel != 0u {
	// 	return;
	// }
	
    if max_dist_ > 1u {
    	// if all adjacent voxels are air, then the dfv is atleast 2
    	if get_voxel(pos + vec3(1, 0, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(0, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(0, 0, -1)) == 0u
    	{ dfv = 2u; }
    }
    if max_dist_ > 2u {
    	// if all voxels with a distance of 2 are air, then the dfv is atleast 3
    	if get_voxel(pos + vec3(2, 0, 0)) == 0u
    	&& get_voxel(pos + vec3(1, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(1, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(1, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(1, 0, -1)) == 0u
    	&& get_voxel(pos + vec3(0, 2, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 1, -1)) == 0u
    	&& get_voxel(pos + vec3(0, 1, 1)) == 0u
    	&& get_voxel(pos + vec3(0, 0, -2)) == 0u
    	&& get_voxel(pos + vec3(0, 0, 2)) == 0u
    	&& get_voxel(pos + vec3(0, -1, -1)) == 0u
    	&& get_voxel(pos + vec3(0, -1, 1)) == 0u
    	&& get_voxel(pos + vec3(0, -2, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, -1)) == 0u
    	&& get_voxel(pos + vec3(-2, 0, 0)) == 0u
    	{ dfv = 3u; }
    }
    if max_dist_ > 3u {
    	if get_voxel(pos + vec3(3, 0, 0)) == 0u
    	&& get_voxel(pos + vec3(2, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(2, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(2, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(2, 0, -1)) == 0u
    	&& get_voxel(pos + vec3(1, 2, 0)) == 0u
    	&& get_voxel(pos + vec3(1, 1, -1)) == 0u
    	&& get_voxel(pos + vec3(1, 1, 1)) == 0u
    	&& get_voxel(pos + vec3(1, 0, -2)) == 0u
    	&& get_voxel(pos + vec3(1, 0, 2)) == 0u
    	&& get_voxel(pos + vec3(1, -1, -1)) == 0u
    	&& get_voxel(pos + vec3(1, -1, 1)) == 0u
    	&& get_voxel(pos + vec3(1, -2, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 3, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 2, 1)) == 0u
    	&& get_voxel(pos + vec3(0, 2, -1)) == 0u
    	&& get_voxel(pos + vec3(0, 1, 2)) == 0u
    	&& get_voxel(pos + vec3(0, 1, -2)) == 0u
    	&& get_voxel(pos + vec3(0, 0, 3)) == 0u
    	&& get_voxel(pos + vec3(0, 0, -3)) == 0u
    	&& get_voxel(pos + vec3(0, -1, 2)) == 0u
    	&& get_voxel(pos + vec3(0, -1, -2)) == 0u
    	&& get_voxel(pos + vec3(0, -2, 1)) == 0u
    	&& get_voxel(pos + vec3(0, -2, -1)) == 0u
    	&& get_voxel(pos + vec3(0, -3, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 2, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 1, -1)) == 0u
    	&& get_voxel(pos + vec3(-1, 1, 1)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, -2)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, 2)) == 0u
    	&& get_voxel(pos + vec3(-1, -1, -1)) == 0u
    	&& get_voxel(pos + vec3(-1, -1, 1)) == 0u
    	&& get_voxel(pos + vec3(-1, -2, 0)) == 0u
    	&& get_voxel(pos + vec3(-2, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(-2, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(-2, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(-2, 0, -1)) == 0u
    	&& get_voxel(pos + vec3(-3, 0, 0)) == 0u
    	{ dfv = 4u; }
    }
    if max_dist_ > 4u {
    	if get_voxel(pos + vec3(4, 0, 0)) == 0u
    	&& get_voxel(pos + vec3(3, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(3, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(3, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(3, 0, -1)) == 0u
    	&& get_voxel(pos + vec3(2, 2, 0)) == 0u
    	&& get_voxel(pos + vec3(2, 1, -1)) == 0u
    	&& get_voxel(pos + vec3(2, 1, 1)) == 0u
    	&& get_voxel(pos + vec3(2, 0, -2)) == 0u
    	&& get_voxel(pos + vec3(2, 0, 2)) == 0u
    	&& get_voxel(pos + vec3(2, -1, -1)) == 0u
    	&& get_voxel(pos + vec3(2, -1, 1)) == 0u
    	&& get_voxel(pos + vec3(2, -2, 0)) == 0u
    	&& get_voxel(pos + vec3(1, 3, 0)) == 0u
    	&& get_voxel(pos + vec3(1, 2, 1)) == 0u
    	&& get_voxel(pos + vec3(1, 2, -1)) == 0u
    	&& get_voxel(pos + vec3(1, 1, 2)) == 0u
    	&& get_voxel(pos + vec3(1, 1, -2)) == 0u
    	&& get_voxel(pos + vec3(1, 0, 3)) == 0u
    	&& get_voxel(pos + vec3(1, 0, -3)) == 0u
    	&& get_voxel(pos + vec3(1, -1, 2)) == 0u
    	&& get_voxel(pos + vec3(1, -1, -2)) == 0u
    	&& get_voxel(pos + vec3(1, -2, 1)) == 0u
    	&& get_voxel(pos + vec3(1, -2, -1)) == 0u
    	&& get_voxel(pos + vec3(1, -3, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 4, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 3, 1)) == 0u
    	&& get_voxel(pos + vec3(0, 3, -1)) == 0u
    	&& get_voxel(pos + vec3(0, 2, 2)) == 0u
    	&& get_voxel(pos + vec3(0, 2, -2)) == 0u
    	&& get_voxel(pos + vec3(0, 1, 3)) == 0u
    	&& get_voxel(pos + vec3(0, 1, -3)) == 0u
    	&& get_voxel(pos + vec3(0, 0, 4)) == 0u
    	&& get_voxel(pos + vec3(0, 0, -4)) == 0u
    	&& get_voxel(pos + vec3(0, -1, 3)) == 0u
    	&& get_voxel(pos + vec3(0, -1, -3)) == 0u
    	&& get_voxel(pos + vec3(0, -2, 2)) == 0u
    	&& get_voxel(pos + vec3(0, -2, -2)) == 0u
    	&& get_voxel(pos + vec3(0, -3, 1)) == 0u
    	&& get_voxel(pos + vec3(0, -3, -1)) == 0u
    	&& get_voxel(pos + vec3(0, -4, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 3, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 2, 1)) == 0u
    	&& get_voxel(pos + vec3(-1, 2, -1)) == 0u
    	&& get_voxel(pos + vec3(-1, 1, 2)) == 0u
    	&& get_voxel(pos + vec3(-1, 1, -2)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, 3)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, -3)) == 0u
    	&& get_voxel(pos + vec3(-1, -1, 2)) == 0u
    	&& get_voxel(pos + vec3(-1, -1, -2)) == 0u
    	&& get_voxel(pos + vec3(-1, -2, 1)) == 0u
    	&& get_voxel(pos + vec3(-1, -2, -1)) == 0u
    	&& get_voxel(pos + vec3(-1, -3, 0)) == 0u
    	&& get_voxel(pos + vec3(-2, 2, 0)) == 0u
    	&& get_voxel(pos + vec3(-2, 1, -1)) == 0u
    	&& get_voxel(pos + vec3(-2, 1, 1)) == 0u
    	&& get_voxel(pos + vec3(-2, 0, -2)) == 0u
    	&& get_voxel(pos + vec3(-2, 0, 2)) == 0u
    	&& get_voxel(pos + vec3(-2, -1, -1)) == 0u
    	&& get_voxel(pos + vec3(-2, -1, 1)) == 0u
    	&& get_voxel(pos + vec3(-2, -2, 0)) == 0u
    	&& get_voxel(pos + vec3(-3, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(-3, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(-3, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(-3, 0, -1)) == 0u
    	&& get_voxel(pos + vec3(-4, 0, 0)) == 0u
    	{ dfv = 5u; }
    }
    if max_dist_ > 5u {
    	if get_voxel(pos + vec3(5, 0, 0)) == 0u
    	&& get_voxel(pos + vec3(4, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(4, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(4, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(4, 0, -1)) == 0u
    	&& get_voxel(pos + vec3(3, 2, 0)) == 0u
    	&& get_voxel(pos + vec3(3, 1, -1)) == 0u
    	&& get_voxel(pos + vec3(3, 1, 1)) == 0u
    	&& get_voxel(pos + vec3(3, 0, -2)) == 0u
    	&& get_voxel(pos + vec3(3, 0, 2)) == 0u
    	&& get_voxel(pos + vec3(3, -1, -1)) == 0u
    	&& get_voxel(pos + vec3(3, -1, 1)) == 0u
    	&& get_voxel(pos + vec3(3, -2, 0)) == 0u
    	&& get_voxel(pos + vec3(2, 3, 0)) == 0u
    	&& get_voxel(pos + vec3(2, 2, 1)) == 0u
    	&& get_voxel(pos + vec3(2, 2, -1)) == 0u
    	&& get_voxel(pos + vec3(2, 1, 2)) == 0u
    	&& get_voxel(pos + vec3(2, 1, -2)) == 0u
    	&& get_voxel(pos + vec3(2, 0, 3)) == 0u
    	&& get_voxel(pos + vec3(2, 0, -3)) == 0u
    	&& get_voxel(pos + vec3(2, -1, 2)) == 0u
    	&& get_voxel(pos + vec3(2, -1, -2)) == 0u
    	&& get_voxel(pos + vec3(2, -2, 1)) == 0u
    	&& get_voxel(pos + vec3(2, -2, -1)) == 0u
    	&& get_voxel(pos + vec3(2, -3, 0)) == 0u
    	&& get_voxel(pos + vec3(1, 4, 0)) == 0u
    	&& get_voxel(pos + vec3(1, 3, 1)) == 0u
    	&& get_voxel(pos + vec3(1, 3, -1)) == 0u
    	&& get_voxel(pos + vec3(1, 2, 2)) == 0u
    	&& get_voxel(pos + vec3(1, 2, -2)) == 0u
    	&& get_voxel(pos + vec3(1, 1, 3)) == 0u
    	&& get_voxel(pos + vec3(1, 1, -3)) == 0u
    	&& get_voxel(pos + vec3(1, 0, 4)) == 0u
    	&& get_voxel(pos + vec3(1, 0, -4)) == 0u
    	&& get_voxel(pos + vec3(1, -1, 3)) == 0u
    	&& get_voxel(pos + vec3(1, -1, -3)) == 0u
    	&& get_voxel(pos + vec3(1, -2, 2)) == 0u
    	&& get_voxel(pos + vec3(1, -2, -2)) == 0u
    	&& get_voxel(pos + vec3(1, -3, 1)) == 0u
    	&& get_voxel(pos + vec3(1, -3, -1)) == 0u
    	&& get_voxel(pos + vec3(1, -4, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 5, 0)) == 0u
    	&& get_voxel(pos + vec3(0, 4, 1)) == 0u
    	&& get_voxel(pos + vec3(0, 4, -1)) == 0u
    	&& get_voxel(pos + vec3(0, 3, 2)) == 0u
    	&& get_voxel(pos + vec3(0, 3, -2)) == 0u
    	&& get_voxel(pos + vec3(0, 2, 3)) == 0u
    	&& get_voxel(pos + vec3(0, 2, -3)) == 0u
    	&& get_voxel(pos + vec3(0, 1, 4)) == 0u
    	&& get_voxel(pos + vec3(0, 1, -4)) == 0u
    	&& get_voxel(pos + vec3(0, 0, 5)) == 0u
    	&& get_voxel(pos + vec3(0, 0, -5)) == 0u
    	&& get_voxel(pos + vec3(0, -1, 4)) == 0u
    	&& get_voxel(pos + vec3(0, -1, -4)) == 0u
    	&& get_voxel(pos + vec3(0, -2, 3)) == 0u
    	&& get_voxel(pos + vec3(0, -2, -3)) == 0u
    	&& get_voxel(pos + vec3(0, -3, 2)) == 0u
    	&& get_voxel(pos + vec3(0, -3, -2)) == 0u
    	&& get_voxel(pos + vec3(0, -4, 1)) == 0u
    	&& get_voxel(pos + vec3(0, -4, -1)) == 0u
    	&& get_voxel(pos + vec3(0, -5, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 4, 0)) == 0u
    	&& get_voxel(pos + vec3(-1, 3, 1)) == 0u
    	&& get_voxel(pos + vec3(-1, 3, -1)) == 0u
    	&& get_voxel(pos + vec3(-1, 2, 2)) == 0u
    	&& get_voxel(pos + vec3(-1, 2, -2)) == 0u
    	&& get_voxel(pos + vec3(-1, 1, 3)) == 0u
    	&& get_voxel(pos + vec3(-1, 1, -3)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, 4)) == 0u
    	&& get_voxel(pos + vec3(-1, 0, -4)) == 0u
    	&& get_voxel(pos + vec3(-1, -1, 3)) == 0u
    	&& get_voxel(pos + vec3(-1, -1, -3)) == 0u
    	&& get_voxel(pos + vec3(-1, -2, 2)) == 0u
    	&& get_voxel(pos + vec3(-1, -2, -2)) == 0u
    	&& get_voxel(pos + vec3(-1, -3, 1)) == 0u
    	&& get_voxel(pos + vec3(-1, -3, -1)) == 0u
    	&& get_voxel(pos + vec3(-1, -4, 0)) == 0u
    	&& get_voxel(pos + vec3(-2, 3, 0)) == 0u
    	&& get_voxel(pos + vec3(-2, 2, 1)) == 0u
    	&& get_voxel(pos + vec3(-2, 2, -1)) == 0u
    	&& get_voxel(pos + vec3(-2, 1, 2)) == 0u
    	&& get_voxel(pos + vec3(-2, 1, -2)) == 0u
    	&& get_voxel(pos + vec3(-2, 0, 3)) == 0u
    	&& get_voxel(pos + vec3(-2, 0, -3)) == 0u
    	&& get_voxel(pos + vec3(-2, -1, 2)) == 0u
    	&& get_voxel(pos + vec3(-2, -1, -2)) == 0u
    	&& get_voxel(pos + vec3(-2, -2, 1)) == 0u
    	&& get_voxel(pos + vec3(-2, -2, -1)) == 0u
    	&& get_voxel(pos + vec3(-2, -3, 0)) == 0u
    	&& get_voxel(pos + vec3(-3, 2, 0)) == 0u
    	&& get_voxel(pos + vec3(-3, 1, -1)) == 0u
    	&& get_voxel(pos + vec3(-3, 1, 1)) == 0u
    	&& get_voxel(pos + vec3(-3, 0, -2)) == 0u
    	&& get_voxel(pos + vec3(-3, 0, 2)) == 0u
    	&& get_voxel(pos + vec3(-3, -1, -1)) == 0u
    	&& get_voxel(pos + vec3(-3, -1, 1)) == 0u
    	&& get_voxel(pos + vec3(-3, -2, 0)) == 0u
    	&& get_voxel(pos + vec3(-4, 1, 0)) == 0u
    	&& get_voxel(pos + vec3(-4, -1, 0)) == 0u
    	&& get_voxel(pos + vec3(-4, 0, 1)) == 0u
    	&& get_voxel(pos + vec3(-4, 0, -1)) == 0u
    	&& get_voxel(pos + vec3(-5, 0, 0)) == 0u
    	{ dfv = 6u; }
    }
	return dfv;
}

@compute @workgroup_size(1, 1, 1)
fn update(@builtin(global_invocation_id) inv_id: vec3<u32>) {
	let pos0 = vec3(i32(inv_id.x * 4u + 0u), i32(inv_id.y), i32(inv_id.z));
	let pos1 = vec3(i32(inv_id.x * 4u + 1u), i32(inv_id.y), i32(inv_id.z));
	let pos2 = vec3(i32(inv_id.x * 4u + 2u), i32(inv_id.y), i32(inv_id.z));
	let pos3 = vec3(i32(inv_id.x * 4u + 3u), i32(inv_id.y), i32(inv_id.z));
	
	let dfv0 = compute_voxel_dfv(pos0);
	let dfv1 = compute_voxel_dfv(pos1);
	let dfv2 = compute_voxel_dfv(pos2);
	let dfv3 = compute_voxel_dfv(pos3);
	
	let int_idx: u32 = (
        u32(pos0.z) * WORLD_W * WORLD_H + 
        u32(pos0.y) * WORLD_W + 
        u32(pos0.x)
    ) / 4u;
	
    sdf_[int_idx] = (dfv3 << 24u) | (dfv2 << 16u) | (dfv1 << 8u) | dfv0;
}
