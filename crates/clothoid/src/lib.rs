use glam::{DVec2, Mat2, Vec2, dvec2};
use std::{f64::consts::{PI, TAU}, mem::replace};

mod fresnel_int;
pub use fresnel_int::*;

pub const SHADER_INCLUDE: &str = include_str!("spirals.wgsl");

fn spiro2_poly(a: f64, b: f64) -> DVec2 {
    const ORDER: i32 = 12;

    let mut p = DVec2::X;
    let mut bell1 = DVec2::X;
    let mut bell2 = DVec2::ZERO;
    let mut d = 0.5;

    for n in 2..=ORDER {
        let bell = (a * bell1 + ((n - 2) as f64) * b * bell2).rotate(DVec2::Y);
        d *= 0.5 / (n as f64);

        if n % 2 == 1 {
            p += 2.0 * d * bell;
        }

        bell2 = bell1;
        bell1 = bell;
    }
    p
}

fn spiro2_subdiv(a: f64, b: f64) -> DVec2 {
    const N: i32 = 256;
    let ds = 1.0 / (N as f64);

    let mut p = DVec2::ZERO;
    for i in 0..N {
        let s = (i as f64 + 0.5) / (N as f64) - 0.5;

        let theta = (a + 0.5 * s * b) * s;
        let tangent = DVec2::from_angle(theta);
        let seg = spiro2_poly(ds * (a + b*s), ds * ds * b);
        p += seg.rotate(tangent);
    }
    ds * p
}

fn fresnel_old(s: f64) -> DVec2 {
    let s2 = s * s;
    let theta = s2 / 4.0;
    let tangent = DVec2::from_angle(theta);
    s * spiro2_subdiv(s2, 2.0 * s2).rotate(tangent)
}

// Raph Levien, From Spiral to Spline, fig 8.3.
// Find (a, b) for spiral segment using endpoint angles CCW from chord.
// th0 is inverted from source (which had it CW)
fn fit_euler_relative(th0: f64, th1: f64) -> (DVec2, DVec2, f64, f64) {
    let a = th1 - th0;

    let mut err_old = th1 + th0;
    let mut b_old = 0.0;
    
    let aa = 1.0 - a / TAU;
    let mut b = 6.0 * (aa * aa * aa) * err_old;
    let mut chord = DVec2::ZERO;

    for i in 0..10 {
        chord = spiro2(a, b);
        let err = (th1 + th0) + 2.0 * chord.to_angle() - 0.25 * b;
        if err.abs() < 1e-9 { break; }
        if err == err_old {
            panic!("got error {} on run {} with chord {} and b {} previously {}", err, i, chord, b, b_old);
        }
        let new_b =  b - (b - b_old) * err / (err - err_old);
        //if i == 9 {
        //    panic!("got last step with error {} (prev {}) for angles {} {}", err, err_old, th0, th1);
        //}
        err_old = err;
        b_old = b;
        b = new_b;
    }
    let curv = a + b * dvec2(-0.5, 0.5);
    return (chord, curv, a, b);
}

fn mod_tau(x: f64) -> f64 {
    (x + PI).rem_euclid(TAU) - PI
}

#[derive(Debug, Clone, Copy)]
pub struct FitEulerResult {
    pub a: f64,
    pub b: f64,

    pub rel_chord: DVec2,
    pub curv: DVec2,
    pub curv_d0: DVec2,
    pub curv_d1: DVec2,
    pub jolt: f64,
    pub jolt_d0: f64,
    pub jolt_d1: f64,
}

fn fit_euler_abs_deriv(p0: DVec2, p1: DVec2, th0: f64, th1: f64) -> FitEulerResult {
    const EPSILON: f64 = 1e-6;

    let chord = p1-p0;
    let chord_len = chord.length();
    let chord_angle = chord.to_angle();
    let rth0 = mod_tau(th0 - chord_angle);
    let rth1 = mod_tau(th1 - chord_angle);
    
    let (rel_chord, rel_curv, a, b) = fit_euler_relative(rth0, rth1);
    let ds = chord_len / rel_chord.length();
    let curv = rel_curv / ds;
    let jolt = b / (ds * ds);

    // finite differences for derivative by tangent angles
    let (rel_chord_0, rel_curv_0, _, b0) = fit_euler_relative(rth0 + EPSILON, rth1);
    let ds0 = chord_len / rel_chord_0.length();
    let curv_0 = rel_curv_0 / ds0;
    let curv_d0 = (curv_0 - curv) / EPSILON;
    let jolt_0 = b0 / (ds0 * ds0);
    let jolt_d0 = (jolt_0 - jolt) / EPSILON;


    let (rel_chord_1, rel_curv_1, _, b1) = fit_euler_relative(rth0, rth1 + EPSILON);
    let ds1 = chord_len / rel_chord_1.length();
    let curv_1 = rel_curv_1 / ds1;
    let curv_d1 = (curv_1 - curv) / EPSILON;
    let jolt_1 = b1 / (ds1 * ds1);
    let jolt_d1 = (jolt_1 - jolt) / EPSILON;

    FitEulerResult { a, b, rel_chord, curv, curv_d0 , curv_d1, jolt, jolt_d0, jolt_d1 }
}

// Adapted from Numerical Recepies
pub fn solve_tridiag(m: &[[f64; 3]], r: &[f64]) -> Vec<f64> {
    let n = m.len();
    let mut u = vec![0.0; n];

    let mut gamma = vec![0.0; n];
    let mut beta = m[0][1];

    u[0] = r[0] / beta;

    for j in 1..n {
        gamma[j] = m[j-1][2] / beta;
        beta = m[j][1] - m[j][0] * gamma[j];
        u[j] = (r[j] - m[j][0]*u[j-1]) / beta;
    }
    for j in (0..(n-1)).rev() {
        u[j] -= gamma[j+1] * u[j +1];
    }
    u
}

fn refine_euler(points: &[DVec2], tangents: &[f64]) -> (Vec<FitEulerResult>, Vec<f64>, Option<Vec<f64>>) {
    let n = points.len();
    let mut fits = Vec::with_capacity(n-1);
    for i in 0..(n-1) {
        fits.push(fit_euler_abs_deriv(points[i], points[i+1], tangents[i], tangents[i+1]));
    }

    let mut jac = Vec::with_capacity(n);
    let mut errs = Vec::with_capacity(n);

    // boundary condition: end segment is circular arc
    errs.push(fits[0].jolt);
    jac.push([0.0, fits[0].jolt_d0, fits[0].jolt_d1]);

    // straight boundary
    //errs.push(fits[0].curv.x);
    //jac.push([0.0, fits[0].curv_d0.x, fits[0].curv_d1.x]);

    for i in 1..(n-1) {
        let j = i-1;
        errs.push(fits[i].curv.x - fits[j].curv.y);
        jac.push([-fits[j].curv_d0.y, fits[i].curv_d0.x - fits[j].curv_d1.y, fits[i].curv_d1.x]);
    }

    errs.push(fits[n-2].jolt);
    jac.push([fits[n-2].jolt_d0, fits[n-2].jolt_d1, 0.0]);
    //errs.push(-fits[n-2].curv.y);
    //jac.push([-fits[n-2].curv_d0.y, -fits[n-2].curv_d1.y, 0.0]);
    
    let maxerr = errs.iter().copied().map(f64::abs).reduce(f64::max).unwrap_or(0.);
    if maxerr < 1e-3 {
        return(fits, errs, None);
    }

    let deltas = solve_tridiag(&jac, &errs);
    let mut new_tans = Vec::with_capacity(n);
    for i in 0..n {
        new_tans.push(tangents[i] - deltas[i]);
    }
    (fits, errs, Some(new_tans))
}

const CONJ: DVec2 = dvec2(1.0, -1.0);

fn mid_circle_tangent(a: DVec2, b: DVec2, c: DVec2) -> DVec2 {
    let ab = (b - a).normalize();
    let bc = (c - b).normalize();
    let ac = (c - a).normalize();

    bc.rotate(ab.rotate(ac * CONJ))
}

fn start_circle_tangent(a: DVec2, b: DVec2, c: DVec2) -> DVec2 {
    let ab = (b - a).normalize();
    let cb = (b - c).normalize();
    let ca = (a - c).normalize();

    ab.rotate(ca.rotate(cb * CONJ))
}

pub fn solve_euler_spline(points: &[DVec2]) -> (Vec<f64>, Vec<FitEulerResult>){
    const MAX_ITER: u32 = 20;
    let n = points.len();

    let mut tangents = vec![0.0; n];
    tangents[0] = start_circle_tangent(points[0],points[1], points[2]).to_angle();
    tangents[n-1] = (-start_circle_tangent(points[n-1],points[n-2], points[n-3])).to_angle();
    for i in 1..(n-1) {
        tangents[i] = mid_circle_tangent(points[i-1], points[i], points[i+1]).to_angle();
    }

    let mut old_errsum = 0.0;
    let mut old_tans = Vec::new();

    for pass in 0..=MAX_ITER {
        let (fits, errs, next) = refine_euler(&points, &tangents);

        let new_tans = match next {
            Some(new_tans) => {
                if pass == MAX_ITER {
                    return (tangents, fits);
                } else {
                    new_tans
                }
            },
            None => {
                return (tangents, fits);
            },
        };

        if pass == 0 {
            old_errsum = errs.iter().copied().map(|x|{x*x}).sum();
            old_tans = replace(&mut tangents, new_tans);
        } else {
            let errsum = errs.iter().copied().map(|x|{x*x}).sum();
            if errsum < old_errsum {
                old_tans = replace(&mut tangents, new_tans);
            } else {
                for i in 0..n {
                    tangents[i] = tangents[i].midpoint(old_tans[i]);
                }
            }
            old_errsum = errsum;
        }

    }
    unreachable!()
}

// Solved Euler spiral spline using f32 values for GPU
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C, align(8))]
pub struct ClothoidSegParams {
    pub start: Vec2,
    pub end: Vec2,
    pub a: f32,
    pub b: f32,
    pub rel_chord: Vec2,
    pub start_tan: f32,
    pub end_tan: f32,
    pub arclen: f32,
    pub arc_start: f32,
}

pub fn stage_euler_spline(points: &[DVec2], tangents: &[f64], fits: &[FitEulerResult]) -> Vec<ClothoidSegParams> {
    let n = points.len();
    let mut out = Vec::new();
    let mut start = 0.0;
    for i in 0..(n-1) {
        let arclen = fits[i].rel_chord.length_recip() * (points[i+1] - points[i]).length();
        out.push(ClothoidSegParams {
            start: points[i].as_vec2(),
            end: points[i+1].as_vec2(),
            a: fits[i].a as f32,
            b: fits[i].b as f32,
            rel_chord: fits[i].rel_chord.as_vec2(),
            start_tan: tangents[i] as f32,
            end_tan: tangents[i+1] as f32,
            arclen: arclen as f32,
            arc_start: start as f32,
        });
        start += arclen;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_tau() {
        for i in (-100)..100 {
            let t = 0.1 * (i as f64);
            let tt = mod_tau(t);
            assert!(tt.abs() <= PI, "t={}: got {}", t, tt);
        }
    }

    #[test]
    fn test_spiro_sinc() {
        for i in 1..100 {
            let t: f64 = 0.1 * (i as f64);

            let from_math = t.sin() / t;
            let spiro_result = spiro2(2.0 * t, 0.0);
            let from_spiro = spiro_result.x;
            let err = (from_spiro - from_math).abs();

            assert!(err < 0.0001, "t={}: got {} from spiro, {} from sinc", t, spiro_result, from_math);
        }
    }

    #[test]
    fn test_spiros_match() {
        for i in (-50)..50 {
            for j in (-50)..50 {
                let a = 0.1 * (i as f64);
                let b = 0.1 * (j as f64);

                let from_sub = spiro2_subdiv(a, b);
                let from_spiro = spiro2(a, b);
                let err = from_sub.distance(from_spiro);

                assert!(err < 0.001, "a={} b={}: got {} from euler integration, {} directly", a, b, from_sub, from_spiro);
            }
        }
    }

    #[test]
    fn test_fresnels() {
        for i in 1..100 {
            let t: f64 = 0.1 * (i as f64);

            let from_a = fresnel_old(t);
            let from_b = simple_fresnel(t);
            let err = (from_a.length() - from_b.length()).abs();

            println!("t={}: {} and {}", t, from_a, from_b);
            assert!(err < 0.0001, "t={}: got {} from spiro and {} directly", t, from_a, from_b);
        }
    }

    #[test]
    fn test_fit_euler_zero() {
        let (chord, curv, a, b) = fit_euler_relative(0.0, 0.0);
        assert!(a.abs() < 0.0001 && b.abs() < 0.0001, "got a={} and b={}", a, b);

        let fit = fit_euler_abs_deriv(dvec2(1.0, 0.0), dvec2(2.0, 0.0), 0.0, 0.0);
        assert!(fit.a.abs() < 0.0001 && fit.b.abs() < 0.0001, "got bad fit {:?}", fit);

        let (chord, curv, a, b) = fit_euler_relative(0.5, -0.5);
        assert!(a.is_finite() && b.abs() < 0.0001, "got a={} and b={}", a, b);

        let (chord, curv, a, b) = fit_euler_relative(0.5, 0.5);
        assert!(a.abs() < 0.0001 && b.is_finite(), "got a={} and b={}", a, b);
    }
}