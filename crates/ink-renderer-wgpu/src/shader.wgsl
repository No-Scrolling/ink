struct QuadOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) colour: vec4<f32>,
}

@vertex
fn quad_vertex(
    @location(0) position: vec2<f32>,
    @location(1) colour: vec4<f32>,
) -> QuadOutput {
    var output: QuadOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
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
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
) -> TextOutput {
    var output: TextOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = uv;
    output.colour = colour;
    return output;
}

@group(0) @binding(0)
var glyph_atlas: texture_2d<f32>;

@group(0) @binding(1)
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
