const PI = 3.1415926535;

fn timesi(a: vec2f) -> vec2f {
    return vec2f(-a.y, a.x);
}

fn cmul(a: vec2f, b: vec2f) -> vec2f {
    return vec2f(a.x * b.x - a.y * b.y, dot(a, b.yx));
}

fn crecip(a: vec2f) -> vec2f {
    return vec2f(a.x, -a.y) / dot(a, a);
}

fn s_poly(t: f32) -> f32 {
    var p = 1.647629463788700E-009;
    p = p * t - 1.522754752581096E-007;
    p = p * t + 8.424748808502400E-006;
    p = p * t - 3.120693124703272E-004;
    p = p * t + 7.244727626597022E-003;
    p = p * t - 9.228055941124598E-002;
    p = p * t + 5.235987735681432E-001;
    return p;
}

fn c_poly(t: f32) -> f32 {
    var p = 1.416802502367354E-008;
    p = p * t - 1.157231412229871E-006;
    p = p * t + 5.387223446683264E-005;
    p = p * t - 1.604381798862293E-003;
    p = p * t + 2.818489036795073E-002;
    p = p * t - 2.467398198317899E-001;
    p = p * t + 9.999999760004487E-001;
    return p;
}


fn f_poly(t: f32) -> f32 {
    var p = -1.903009855649792E+012;
    p = p * t + 1.355942388050252E+011;
    p = p * t - 4.158143148511033E+009;
    p = p * t + 7.343848463587323E+007;
    p = p * t - 8.732356681548485E+005;
    p = p * t + 8.560515466275470E+003;
    p = p * t - 1.032877601091159E+002;
    p = p * t + 2.999401847870011E+000;
    return p;
}

fn g_poly(t: f32) -> f32 {
    var p = -1.860843997624650E+011;
    p = p * t + 1.278350673393208E+010;
    p = p * t - 3.779387713202229E+008;
    p = p * t + 6.492611570598858E+006;
    p = p * t - 7.787789623358162E+004;
    p = p * t + 8.602931494734327E+002;
    p = p * t - 1.493439396592284E+001;
    p = p * t + 9.999841934744914E-001;
    return p;
}

// integral of E^i(PI/2 * t^2)
fn norm_fresnel(x: f32) -> vec2f {
    let x2 = x * x;
    if x2 < 2.5625 {
        let x4 = x2 * x2;
        let c = x * c_poly(x4);
        let s = x * x2 * s_poly(x4);
        return vec2f(c, s);
    } else if x > 36974.0 {
        return sign(x) * vec2f(0.5, 0.5);
    } else {
        let u = 1.0 / (PI * x2);
        let u2 = u * u;
        let g = u * g_poly(u2);
        let f = 1.0 - u2 * f_poly(u2);
        let gf = vec2f(g, f);

        let theta = 0.5 * PI * x2;
        let cs = vec2f(cos(theta), sin(theta));
        let t = PI * abs(x);
        return sign(x) * (0.5 - cmul(gf, cs)/t);
    }
}

fn simple_fresnel(x: f32) -> vec2f {
    let norm = sqrt(PI / 2.0);
    return norm_fresnel(x / norm) * norm;
}

// Numerically-stable version of two-parameter spiro dunction
fn spiro2(a: f32, b: f32) -> vec2f {
    let aa = abs(a);
    let ab = abs(b);
    if ab < 5e-8 {
        // special cases to avoid division by zero
        if a == 0.0 {
            return vec2f(1, 0);
        }
        return vec2f(sin(0.5 * a) / (0.5 * a), 0.0);
    }
    let sb = sqrt(ab * PI);
    let tmin = (aa - 0.5 * ab) / sb;
    if tmin > 2 {
        // with both ends of the fresnel integral falling into the same asymptotic case,
        // inline and rearrange calculations algebraically to avoid floating-point error on small values of b.
        let t2m = a * a / ab  + ab/4;

        let u0 = 1 / (t2m - aa);
        let u1 = 1 / (t2m + aa);
        let u02 = u0 * u0;
        let u12 = u1 * u1;

        let gf0 = vec2f(u0 * g_poly(u02), 1 - u02 * f_poly(u02));
        let gf1 = vec2f(u1 * g_poly(u12), 1 - u12 * f_poly(u12));

        let th0 = ab / 8 - 0.5 * aa;
        let th1 = ab / 8 + 0.5 * aa;
        let dcs0 = vec2f(cos(th0), sin(th0));
        let dcs1 = vec2f(cos(th1), sin(th1));
        let p0 = cmul(gf0, dcs0) / (aa - ab/2);
        let p1 = cmul(gf1, dcs1) / (aa + ab / 2);
        return (p0 - p1) * vec2f(1.0, sign(b));
    } else {
        // call fresnel integral normally
        let t0 = (a - 0.5 * ab) / sb;
        let t1 = (a + 0.5 * ab) / sb;
        let p0 = norm_fresnel(t0);
        let p1 = norm_fresnel(t1);
        let chord = (p1 - p0) / (t1 - t0);
        let theta = 0.5 * a * a / ab;
        let m = vec2f(cos(theta), -sin(theta));
        return cmul(chord, m) * vec2f(1.0, sign(b));
    }
}

// same as the rust version
struct ClothoidSegParams {
    start: vec2f,
    end: vec2f,
    a: f32,
    b: f32,
    rel_chord: vec2f,
    start_tan: f32,
    end_tan: f32,
    arclen: f32,
    arc_start: f32,
}

// get a specific point in a clothoid segment
fn get_clothoid_seg_point(seg: ClothoidSegParams, t: f32) -> vec2f {
    let s = (t - 1) / 2;
    let part_seg = spiro2((seg.a + seg.b * s) * t, seg.b * t * t);
    let theta = (seg.a + 0.5 * s * seg.b) * s;
    let tangent = vec2f(cos(theta), sin(theta));
    let part_p = cmul(part_seg, tangent) * t;

    let chord = seg.end - seg.start;
    let p = seg.start + cmul(chord, cmul(part_p, crecip(seg.rel_chord)));
    return p;
}

