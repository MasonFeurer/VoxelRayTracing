@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var tex_s: sampler;

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
	return textureSample(tex, tex_s, in.tex_coord);
}
