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
    @location(2) circle: vec2<f32>,
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
    output.circle = vec2<f32>(0.0);
    if colour.z < 0.0 && colour.w > 0.0 {
        let uv = output.uv;
        if colour.x == 90.0 {
            output.uv = vec2<f32>(uv.y, 1.0 - uv.x);
        } else if colour.x == 180.0 {
            output.uv = vec2<f32>(1.0 - uv.x, 1.0 - uv.y);
        } else if colour.x == 270.0 {
            output.uv = vec2<f32>(1.0 - uv.y, uv.x);
        }
        output.colour = vec4<f32>(1.0);
    }
    if colour.w < 0.0 {
        output.circle = (position - colour.xy) / abs(colour.zw);
    }
    return output;
}

@group(1) @binding(0)
var glyph_atlas: texture_2d<f32>;

@group(1) @binding(1)
var glyph_sampler: sampler;

@fragment
fn text_fragment(input: TextOutput) -> @location(0) vec4<f32> {
    if input.colour.w < 0.0 {
        let distance = length(input.circle);
        let edge = max(fwidth(distance), 0.001);
        // A 72-unit avatar reserves 3 units each for the gap and unread ring.
        let portrait_radius = 30.0 / 36.0;
        let ring_inner = 33.0 / 36.0;
        let portrait = 1.0 - smoothstep(portrait_radius - edge, portrait_radius, distance);
        let ring = smoothstep(ring_inner - edge, ring_inner, distance)
            * (1.0 - smoothstep(1.0 - edge, 1.0, distance));
        let image_offset = vec2<f32>(input.circle.x, -input.circle.y) * (1.0 / portrait_radius - 1.0);
        let uv = input.uv + image_offset * fwidth(input.uv) / max(fwidth(input.circle), vec2<f32>(0.0001));
        let pixel = textureSample(glyph_atlas, glyph_sampler, uv);
        let outline = select(0.0, ring, input.colour.z < 0.0);
        return vec4<f32>(mix(pixel.rgb, vec3<f32>(1.0), outline), pixel.a * portrait + outline);
    }
    return textureSample(glyph_atlas, glyph_sampler, input.uv) * input.colour;
}

struct LineOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) pixel: vec2<f32>,
    @location(1) @interpolate(flat) ends: vec4<f32>,
    @location(2) @interpolate(flat) clip: vec4<f32>,
    @location(3) @interpolate(flat) colour: vec4<f32>,
    @location(4) @interpolate(flat) radius: f32,
}

// Covers the segment and its round ends, plus a pixel for the antialiased edge.
@vertex
fn line_vertex(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) ends: vec4<f32>,
    @location(1) clip: vec4<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) shape: vec4<f32>,
) -> LineOutput {
    let corner = unit_corner(vertex_index);
    let start = ends.xy;
    let span = ends.zw - start;
    let span_length = length(span);
    let along = select(vec2<f32>(1.0, 0.0), span / span_length, span_length > 0.0);
    let across = vec2<f32>(-along.y, along.x);
    let reach = shape.x + 1.0;
    let pixel = start + along * (corner.x * (span_length + reach * 2.0) - reach) + across * (corner.y * 2.0 - 1.0) * reach;
    var output: LineOutput;
    output.position = vec4<f32>(pixel.x * shape.y - 1.0, 1.0 - pixel.y * shape.z, 0.0, 1.0)
        + vec4<f32>(transform.translation.xy, 0.0, 0.0);
    output.pixel = pixel;
    output.ends = ends;
    output.clip = clip;
    output.colour = colour;
    output.radius = shape.x;
    return output;
}

@fragment
fn line_fragment(input: LineOutput) -> @location(0) vec4<f32> {
    let pixel = input.pixel;
    if any(pixel < input.clip.xy) || any(pixel >= input.clip.zw) {
        discard;
    }
    let span = input.ends.zw - input.ends.xy;
    let along = clamp(dot(pixel - input.ends.xy, span) / max(dot(span, span), 0.0001), 0.0, 1.0);
    let distance = length(pixel - input.ends.xy - span * along);
    let coverage = clamp(input.radius + 0.5 - distance, 0.0, 1.0);
    return vec4<f32>(input.colour.rgb, input.colour.a * coverage);
}
