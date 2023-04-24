@group(0) @binding(0) var<uniform> model_mat: mat4x4<f32>;
@group(0) @binding(1) var<uniform> view_mat: mat4x4<f32>;
@group(0) @binding(2) var<uniform> proj_mat: mat4x4<f32>;

struct VsInput {
	@location(0) pos: vec3<f32>,
	@location(1) color: vec4<f32>,
}

struct FsInput {
	@builtin(position) pos: vec4<f32>,
	@location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VsInput) -> FsInput {
	var result: FsInput;
	result.pos = proj_mat * view_mat * model_mat * vec4(in.pos, 1.0);
	result.color = in.color;
	return result;
}

@fragment
fn fs_main(in: FsInput) -> @location(0) vec4<f32> {
	return in.color;
}
