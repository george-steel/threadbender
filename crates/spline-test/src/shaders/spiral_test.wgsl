#include spirals.wgsl
#include global.wgsl

struct VSOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) color: vec4f,
}

@group(0) @binding(0) var<uniform> view: Viewport;

@vertex fn spiral_test_vert(@builtin(vertex_index) vert: u32, @builtin(instance_index) inst: u32) -> VSOut {
    let s = 0.01 * f32(vert) * select(1.0, -1.0, inst == 0);

    let world_pos = 10 * simple_fresnel(s);
    let clip_pos = view.scales * world_pos + view.trans;
    
    var out: VSOut;
    out.clip_pos = vec4f(clip_pos, 0, 1);
    out.color = vec4f(1, 0, 1, 1);
    return out;
}

@fragment fn spiral_test_frag(v: VSOut) -> @location(0) vec4f {
    return v.color;
}
