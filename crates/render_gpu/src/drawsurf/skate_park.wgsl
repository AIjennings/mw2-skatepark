#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vertex(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip_position = view.clip_from_world * vec4(in.position, 1.0);
    out.normal = in.normal;
    out.color = in.color;
    out.uv = in.uv;
    return out;
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    let sun_dir = normalize(vec3<f32>(0.35, 0.45, 0.82));
    let n = normalize(in.normal);
    let diff = max(dot(n, sun_dir), 0.0);
    let ambient = 0.42;
    let light = ambient + diff * 0.58;

    // Subtle skate surface lines
    let gx = abs(fract(in.uv.x * 2.0) - 0.5);
    let gy = abs(fract(in.uv.y * 2.0) - 0.5);
    let line_factor = select(1.0, 0.88, gx < 0.02 || gy < 0.02);

    let lit_color = in.color.rgb * light * line_factor;
    return vec4<f32>(lit_color, in.color.a);
}
