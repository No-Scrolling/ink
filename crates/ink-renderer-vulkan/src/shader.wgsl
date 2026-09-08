struct QuadOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) colour: vec4<f32>,
}

struct Transform {
    translation: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> transform: Transform;

fn unit_corner(vertex_index: u32) -> vec2<f32> {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    return corners[vertex_index];
}

@vertex
fn quad_vertex(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) colour: vec4<f32>,
) -> QuadOutput {
    let corner = unit_corner(vertex_index);
    let position = rect.xy + corner * (rect.zw - rect.xy);
    var output: QuadOutput;
    output.position = vec4<f32>(position + transform.translation.xy, 0.0, 1.0);
    output.colour = colour;
    return output;
}

@fragment
fn quad_fragment(input: QuadOutput) -> @location(0) vec4<f32> {
    return input.colour;
}

struct TextOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
}

@vertex
fn text_vertex(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) uv_rect: vec4<f32>,
    @location(2) colour: vec4<f32>,
) -> TextOutput {
    let corner = unit_corner(vertex_index);
    let position = rect.xy + corner * (rect.zw - rect.xy);
    var output: TextOutput;
    output.position = vec4<f32>(position + transform.translation.xy, 0.0, 1.0);
    output.uv = uv_rect.xy + corner * (uv_rect.zw - uv_rect.xy);
    output.colour = colour;
    return output;
}

@group(1) @binding(0)
var glyph_atlas: texture_2d<f32>;

@group(1) @binding(1)
var glyph_sampler: sampler;

@fragment
fn text_fragment(input: TextOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(glyph_atlas, glyph_sampler, input.uv).r;
    return vec4<f32>(input.colour.rgb, input.colour.a * coverage);
}

@fragment
fn image_fragment(input: TextOutput) -> @location(0) vec4<f32> {
    return textureSample(glyph_atlas, glyph_sampler, input.uv);
}
