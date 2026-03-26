#include spirals.wgsl
#include global.wgsl

override LINES_PER_SEG: u32 = 100;

struct VSOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) color: vec4f,
}

@group(0) @binding(0) var<uniform> view: Viewport;
@group(1) @binding(0) var<storage, read> spline: array<ClothoidSegParams>;

@vertex fn spline_plot_vert(@builtin(vertex_index) vert: u32, @builtin(instance_index) inst: u32) -> VSOut {
    let t = f32(vert) / f32(LINES_PER_SEG);
    let seg = spline[inst];
    let world_pos = get_clothoid_seg_point(seg, t);

    let clip_pos = view.scales * world_pos + view.trans;
    
    var out: VSOut;
    out.clip_pos = vec4f(clip_pos, 0, 1);
    out.color = vec4f(0, 1, 1, 1);
    return out;
}

@fragment fn spline_plot_frag(v: VSOut) -> @location(0) vec4f {
    return v.color;
}
