@group(0) @binding(0) var screen_sampler: sampler;
@group(0) @binding(1) var color_buffer: texture_2d<f32>;

struct VertexOutput {
	@builtin(position) pos: vec4<f32>,
	@location(0) tex_coord: vec2<f32>,
}

@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> VertexOutput {
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
	
	var out: VertexOutput;
	out.pos = vec4(positions[index], 0.0, 1.0);
	out.tex_coord = tex_coords[index];
	return out;
}

@fragment
fn fragment_main(@location(0) tex_coord: vec2<f32>) -> @location(0) vec4<f32> {
	return textureSample(color_buffer, screen_sampler, tex_coord);
}
