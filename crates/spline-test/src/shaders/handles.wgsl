#include global.wgsl

struct DisplayHandle {
    pos: vec2f,
    radius: f32,
    fill_color: pack4h,
    line_color: pack4h,
}

const quad_vert: array<vec2f, 6> = array(
    vec2f(-1, 1),
    vec2f(-1, -1),
    vec2f(1, -1),
    vec2f(-1, 1),
    vec2f(1, -1),
    vec2f(1, 1),
);

struct HandlesVSOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) @interpolate(linear,sample) local_pos: vec2f,
    @location(1) @interpolate(flat,either) radius: f32,
    @location(2) @interpolate(flat,either) outline_color: vec4f,
    @location(3) @interpolate(flat,either) fill_color: vec4f,
}

@group(0) @binding(0) var<uniform> view: Viewport;
@group(1) @binding(0) var<storage,read> handles: array<DisplayHandle>;

@vertex fn handles_vert(@builtin(vertex_index) vert: u32) -> HandlesVSOut {
    let h = handles[vert / 6];
    let corner = quad_vert[vert % 6];

    let radius = h.radius * view.css_ratio;
    let local_pos = corner * radius;
    let clip_center = view.scales * h.pos + view.trans;
    let clip_pos = clip_center + 2 * local_pos / vec2f(view.px_size);

    var out: HandlesVSOut;
    out.clip_pos = vec4f(clip_pos, 0, 1);
    out.local_pos = local_pos;
    out.radius = radius;
    out.outline_color = unpack4h(h.line_color);
    out.fill_color = unpack4h(h.fill_color);
    return out;
}

@fragment fn handles_frag(vs: HandlesVSOut) -> @location(0) vec4f {
    let d = length(vs.local_pos);
    if d > vs.radius {
        discard;
        return vec4f(0);
    } else if d > vs.radius - 1 {
        return vs.outline_color;
    } else {
        return vs.fill_color;
    }
}