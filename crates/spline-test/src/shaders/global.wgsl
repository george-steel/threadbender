struct Viewport {
    scales: vec2f,
    trans: vec2f,
    sw: vec2f,
    ne: vec2f,
    px_size: vec2u,
    css_ratio: f32,
    pad0: u32
}

// packed as vec4h;
alias pack4h = vec2u;
fn unpack4h(v: pack4h) -> vec4f {
    return vec4f(unpack2x16float(v.x), unpack2x16float(v.y));
}

fn unpack4h_premul(v: pack4h) -> vec4f {
    let postmul = vec4f(unpack2x16float(v.x), unpack2x16float(v.y));
    return vec4f(postmul.xyz * postmul.w, postmul.w);
}

