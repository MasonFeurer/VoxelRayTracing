@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var tex_s: sampler;
@group(0) @binding(2) var<uniform> screen_size_: vec2<f32>;
@group(0) @binding(3) var<uniform> crosshair_: Crosshair;

struct Crosshair {
	color: vec4<f32>,
	// 0 = off, 1 = dot, 2 = cross
	style: u32,
	size: f32,
}

struct FsInput {
	@builtin(position) pos: vec4<f32>,
	@location(0) tex_coord: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> FsInput {
	var positions = array<vec2<f32>, 6>(
		vec2(1.0, 1.0),
		vec2(1.0, -1.0),
		vec2(-1.0, -1.0),
		vec2(1.0, 1.0),
		vec2(-1.0, -1.0),
		vec2(-1.0, 1.0),
	);
	var tex_coords = array<vec2<f32>, 6>(
		vec2(1.0, 0.0),
		vec2(1.0, 1.0),
		vec2(0.0, 1.0),
		vec2(1.0, 0.0),
		vec2(0.0, 1.0),
		vec2(0.0, 0.0),
	);
	
	var out: FsInput;
	out.pos = vec4(positions[index], 0.0, 1.0);
	out.tex_coord = tex_coords[index];
	return out;
}

@fragment
fn fs_main(in: FsInput) -> @location(0) vec4<f32> {
	let screen_pos = in.tex_coord * screen_size_;
	let screen_center = screen_size_ * 0.5;
	
	var crosshair_mask: f32 = 0.0;
	if crosshair_.style == 1u { // dot
		crosshair_mask = f32(distance(screen_center, screen_pos) < crosshair_.size) * crosshair_.color.a;
	}
	if crosshair_.style == 2u { // cross
		let diff = vec2(abs(screen_center.x - screen_pos.x), abs(screen_center.y - screen_pos.y));
		let w = crosshair_.size * 0.25;
		
		crosshair_mask = f32(
			    (diff.x < crosshair_.size && diff.y < w)
			 || (diff.y < crosshair_.size && diff.x < w)
		) * crosshair_.color.a;
	}
	return 
		textureSample(tex, tex_s, in.tex_coord) * (1.0 - crosshair_mask) +
		vec4(crosshair_.color.rgb, 1.0) * crosshair_mask
	;
}
