// Adapted from Cephes Math Library

use std::f64::consts::PI;

use glam::{DVec2, dvec2};

fn s_poly(t: f64) -> f64 {
    let mut p = 1.647629463788700E-009;
    p = p * t - 1.522754752581096E-007;
    p = p * t + 8.424748808502400E-006;
    p = p * t - 3.120693124703272E-004;
    p = p * t + 7.244727626597022E-003;
    p = p * t - 9.228055941124598E-002;
    p = p * t + 5.235987735681432E-001;
    p
}

fn c_poly(t: f64) -> f64 {
    let mut p = 1.416802502367354E-008;
    p = p * t - 1.157231412229871E-006;
    p = p * t + 5.387223446683264E-005;
    p = p * t - 1.604381798862293E-003;
    p = p * t + 2.818489036795073E-002;
    p = p * t - 2.467398198317899E-001;
    p = p * t + 9.999999760004487E-001;
    p
}


fn f_poly(t: f64) -> f64 {
    let mut p = -1.903009855649792E+012;
    p = p * t + 1.355942388050252E+011;
    p = p * t - 4.158143148511033E+009;
    p = p * t + 7.343848463587323E+007;
    p = p * t - 8.732356681548485E+005;
    p = p * t + 8.560515466275470E+003;
    p = p * t - 1.032877601091159E+002;
    p = p * t + 2.999401847870011E+000;
    p
}

fn g_poly(t: f64) -> f64 {
    let mut p = -1.860843997624650E+011;
    p = p * t + 1.278350673393208E+010;
    p = p * t - 3.779387713202229E+008;
    p = p * t + 6.492611570598858E+006;
    p = p * t - 7.787789623358162E+004;
    p = p * t + 8.602931494734327E+002;
    p = p * t - 1.493439396592284E+001;
    p = p * t + 9.999841934744914E-001;
    p
}

// integral of E^i(PI/2 * t^2)
pub fn norm_fresnel(x: f64) -> DVec2 {
    let x2 = x * x;
    if x2 < 2.5625 {
        let x4 = x2 * x2;
        let c = x * c_poly(x4);
        let s = x * x2 * s_poly(x4);
        dvec2(c, s)
    } else if x > 36974.0 {
        x.signum() * dvec2(0.5, 0.5)
    } else {
        let u = 1.0 / (PI * x2);
        let u2 = u * u;
        let f = 1.0 - u2 * f_poly(u2);
        let g = u * g_poly(u2);

        let theta = 0.5 * PI * x2;
        let c = theta.cos();
        let s = theta.sin();
        let t = PI * x.abs();
        let cc = 0.5 + (f * s - g * c)/t;
        let ss = 0.5 - (f * c + g * s)/t;
        x.signum() * dvec2(cc, ss)
    }
}

pub fn simple_fresnel(x: f64) -> DVec2 {
    let norm = (PI / 2.0).sqrt();
    norm_fresnel(x / norm) * norm
}

