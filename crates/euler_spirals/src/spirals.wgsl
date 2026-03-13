
fn timesi(a: vec2f) -> vec2f {
    return vec2f(-a.y, a.x);
}

fn cmul(a: vec2f, b: vec2f) -> vec2f {
    return vec2f(a.x * b.x - a.y * b.y, dot(a, b.yx));
}

fn spiro2_poly(a: f32, b: f32) -> vec2f {
    var p = vec2f(1, 0);
    var old1 = vec2f(1, 0);
    var old2 = vec2f(0, 0);
    var d = 1.0;

    for (var n = 2; n < 11; n += 2) {
        let belln = timesi(a * old1 + f32(n-2) * b * old2);
        let belln1 = timesi(a * belln + f32(n-1) * b * old1);

        d = d / f32(4 * n * n+1);

        p += d * belln1;

        old1 = belln1;
        old2 = belln;
    }
    return p;
}

fn spiro2(a: f32, b: f32, n_subdiv: u32) -> vec2f {
    var p = vec2f(0);
    let ds = 1 / f32(n_subdiv);
    for (var i = 0u; i < n_subdiv; i++) {
        let s = (f32(i) + 0.5) * ds - 0.5;

        let theta = (a + 0.5 * s * b) * s;
        let tangent = vec2f(cos(theta), sin(theta));
        let seg = spiro2_poly((a + b * s) * ds, b * ds * ds);
        p += cmul(seg, tangent) * ds;
    }
    return p;
}

fn fresnel_int(s: f32) -> vec2f {
    let s2 = s * s;
    let theta = 0.125 * s2;
    let tangent = vec2f(cos(theta), sin(theta));
    let seg = spiro2(0.5 * s2, s2, 64);
    return s * cmul(seg, tangent);
}
