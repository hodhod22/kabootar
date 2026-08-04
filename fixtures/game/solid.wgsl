struct FrameUniforms {
    view_proj: mat4x4<f32>,
}

struct MaterialUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    uv_xform: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(1) @binding(0) var<uniform> mat: MaterialUniforms;

struct VertexIn {
    @location(0) position: vec3<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = frame.view_proj * mat.model * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Slight green tint so hot-reload content change is observable via hash.
    return vec4<f32>(mat.color.r, mat.color.g * 0.95, mat.color.b, mat.color.a);
}
