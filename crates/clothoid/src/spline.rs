use glam::{DVec2, Vec2, dvec2};
use std::{f64::consts::{PI, TAU}, mem::replace};

pub use crate::fresnel::*;
use crate::util::{solve_cyclic_tridiag, solve_tridiag};

// Raph Levien, From Spiral to Spline, fig 8.3.
// Find (a, b) for spiral segment using endpoint angles CCW from chord.
// th0 is inverted from Levien's version (which had it CW)
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

// Fits an Euler spiral segment based on endpoints and endpoint-space tangents CCW from +X 
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

// Measures error and calculates new tangents using a single round of Newton iteration
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

    for i in 1..(n-1) {
        let j = i-1;
        errs.push(fits[i].curv.x - fits[j].curv.y);
        jac.push([-fits[j].curv_d0.y, fits[i].curv_d0.x - fits[j].curv_d1.y, fits[i].curv_d1.x]);
    }

    errs.push(fits[n-2].jolt);
    jac.push([fits[n-2].jolt_d0, fits[n-2].jolt_d1, 0.0]);
    
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

// Measures error and calculates new tangents using a single round of Newton iteration
fn refine_euler_cyclic(points: &[DVec2], tangents: &[f64]) -> (Vec<FitEulerResult>, Vec<f64>, Option<Vec<f64>>) {
    let n = points.len();
    let mut fits = Vec::with_capacity(n-1);
    for i in 0..(n) {
        let j = (i + 1) % n;
        fits.push(fit_euler_abs_deriv(points[i], points[j], tangents[i], tangents[j]));
    }

    let mut jac = Vec::with_capacity(n);
    let mut errs = Vec::with_capacity(n);

    for i in 0..n {
        let j = (i + n - 1) % n;
        errs.push(fits[i].curv.x - fits[j].curv.y);
        jac.push([-fits[j].curv_d0.y, fits[i].curv_d0.x - fits[j].curv_d1.y, fits[i].curv_d1.x]);
    }
    
    let maxerr = errs.iter().copied().map(f64::abs).reduce(f64::max).unwrap_or(0.);
    if maxerr < 1e-3 {
        return(fits, errs, None);
    }

    let deltas = solve_cyclic_tridiag(jac, &errs);
    let mut new_tans = Vec::with_capacity(n);
    for i in 0..n {
        new_tans.push(tangents[i] - deltas[i]);
    }
    (fits, errs, Some(new_tans))
}

const CONJ: DVec2 = dvec2(1.0, -1.0);

// tangent oc circumcircle of a,b,c at b
fn mid_circle_tangent(a: DVec2, b: DVec2, c: DVec2) -> DVec2 {
    let ab = (b - a).normalize();
    let bc = (c - b).normalize();
    let ac = (c - a).normalize();

    // tangent of circumcircle using alternate segment theorem
    bc.rotate(ab.rotate(ac * CONJ))
}

fn start_circle_tangent(a: DVec2, b: DVec2, c: DVec2) -> DVec2 {
    let ab = (b - a).normalize();
    let cb = (b - c).normalize();
    let ca = (a - c).normalize();

    ab.rotate(ca.rotate(cb * CONJ))
}

pub fn solve_clothoid_section_with_start(points: &[DVec2], mut tangents: Vec<f64>, cyclic: bool) -> (Vec<f64>, Vec<FitEulerResult>) {
    const MAX_ITER: u32 = 20;
    let n = points.len();

    let mut old_errsum = 0.0;
    let mut old_tans = Vec::new();

    for pass in 0..=MAX_ITER {
        let (fits, errs, next) = if cyclic {
            refine_euler_cyclic(&points, &tangents)
        } else {
            refine_euler(&points, &tangents)
        };

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
            // check if the last iteration made total error worse
            let errsum = errs.iter().copied().map(|x|{x*x}).sum();
            if errsum < old_errsum {
                // if not, update tangents with new ones from this iteration
                old_tans = replace(&mut tangents, new_tans);
            } else {
                // if it got worse, abandon this round and backtrack halfway to escape chaos orbit
                for i in 0..n {
                    tangents[i] = tangents[i].midpoint(old_tans[i]);
                }
            }
            old_errsum = errsum;
        }

    }
    unreachable!()
}

pub struct SolvedClothoidSeg {
    pub a: f64,
    pub b: f64,
    pub start_tan: f64,
    pub end_tan: f64,
    pub rel_chord: DVec2,
}

pub fn solve_clothoid_section(points: &[DVec2], cyclic: bool) -> Vec<SolvedClothoidSeg> {
    let n = points.len();
    if n == 0 {
        return Vec::new();
    } else if n == 1 {
        return Vec::new();
    } else if n == 2 {
        let v = points[1] - points[0];
        let dir = v.y.atan2(v.x);
        return vec![SolvedClothoidSeg {
            a: 0.0,
            b: 0.0,
            start_tan: dir,
            end_tan: dir,
            rel_chord: DVec2::X,
        }]
    }

    // Start with tangents based on circle spline.
    // This converges better than Catmull-Rom tangents
    let mut start_tans = vec![0.0; n];
    if cyclic {
        start_tans[0] = mid_circle_tangent(points[n-1], points[0], points[1]).to_angle();
        start_tans[n-1] = mid_circle_tangent(points[n-2], points[n-1], points[0]).to_angle();
    } else {
        start_tans[0] = start_circle_tangent(points[0], points[1], points[2]).to_angle();
        start_tans[n-1] = (-start_circle_tangent(points[n-1], points[n-2], points[n-3])).to_angle();
    }
    for i in 1..(n-1) {
        start_tans[i] = mid_circle_tangent(points[i-1], points[i], points[i+1]).to_angle();
    }

    let (tangents, fits) = solve_clothoid_section_with_start(points, start_tans, cyclic);

    let mut out = Vec::with_capacity(fits.len());
    for (i, fit) in fits.iter().enumerate() {
        out.push(SolvedClothoidSeg {
            a: fit.a,
            b: fit.b,
            start_tan: tangents[i],
            end_tan: tangents[(i+1) % n],
            rel_chord: fit.rel_chord,
        })
    }
    out
}

// Solved Euler spiral segment using f32 values for use in GPU buffers for rendering
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C, align(8))]
pub struct ClothoidSegGPUParams {
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

pub fn stage_clothoid_params(points: &[DVec2], solution: &[SolvedClothoidSeg]) -> Vec<ClothoidSegGPUParams> {
    let mut traveled = 0.0;
    let mut out = Vec::new();
    let n = points.len();
    for (i, seg) in solution.iter().enumerate() {
        let start = points[i];
        let end = points[(i+1) % n];
        let length = start.distance(end) / seg.rel_chord.length();
        out.push(ClothoidSegGPUParams {
            start: start.as_vec2(),
            end: end.as_vec2(),
            a: seg.a as f32,
            b: seg.b as f32,
            rel_chord: seg.rel_chord.as_vec2(),
            start_tan: seg.start_tan as f32,
            end_tan: seg.end_tan as f32,
            arclen: length as f32,
            arc_start: traveled as f32,
        });
        traveled += length;
    }
    out
}

// Convert a solution from solve_clothoid_spline to a single ClothoidDegParams buffer.
pub fn stage_clothoid_spline(points: &[DVec2], tangents: &[f64], fits: &[FitEulerResult]) -> Vec<ClothoidSegGPUParams> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = 0.0;
    for i in 0..(n-1) {
        let arclen = fits[i].rel_chord.length_recip() * (points[i+1] - points[i]).length();
        out.push(ClothoidSegGPUParams {
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
    fn test_fit_euler_zero() {
        let (_, _, a, b) = fit_euler_relative(0.0, 0.0);
        assert!(a.abs() < 0.0001 && b.abs() < 0.0001, "got a={} and b={}", a, b);

        let fit = fit_euler_abs_deriv(dvec2(1.0, 0.0), dvec2(2.0, 0.0), 0.0, 0.0);
        assert!(fit.a.abs() < 0.0001 && fit.b.abs() < 0.0001, "got bad fit {:?}", fit);

        let (_, _, a, b) = fit_euler_relative(0.5, -0.5);
        assert!(a.is_finite() && b.abs() < 0.0001, "got a={} and b={}", a, b);

        let (_, _, a, b) = fit_euler_relative(0.5, 0.5);
        assert!(a.abs() < 0.0001 && b.is_finite(), "got a={} and b={}", a, b);
    }
}