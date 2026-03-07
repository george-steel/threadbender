#include global.wgsl

struct GridUniforms {
    line_spacing: f32,
    major_every: u32,
    line_color: pack4h,
    major_color: pack4h,
    axis_color: pack4h,
    background_color: pack4h,
}

@group(0) @binding(0) var<uniform> view: Viewport;
@group(1) @binding(0) var<uniform> params: GridUniforms;

struct GridVSOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) color: vec4f,
}

@vertex fn grid_line_vert(@builtin(vertex_index) vert: u32, @builtin(instance_index) inst: u32) -> GridVSOut {
    let start = select(view.sw.x, view.sw.y, inst != 0);
    let offset = i32(ceil(start / params.line_spacing));
    let line = (i32(vert) / 2) + offset;
    let line_pos = f32(line) * params.line_spacing;

    var world_pos: vec2f;
    if inst == 0 {
        world_pos = vec2f(line_pos, select(view.sw.y, view.ne.y, (vert % 2) == 0));
    } else {
        world_pos = vec2f(select(view.sw.x, view.ne.x, (vert % 2) == 0), line_pos);
    }
    let clip_pos = view.scales * world_pos + view.trans;

    var colorh = params.line_color;
    if line == 0 {
        colorh = params.axis_color;
    } else if (line % i32(params.major_every)) == 0 {
        colorh = params.major_color;
    }

    var out: GridVSOut;
    out.clip_pos = vec4f(clip_pos, 1.0, 1.0);
    out.color = unpack4h_premul(colorh);
    return out;
}

@fragment fn grid_line_frag(v: GridVSOut) -> @location(0) vec4f {
    return v.color;
}