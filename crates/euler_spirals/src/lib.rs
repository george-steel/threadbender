use glam::DVec2;

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

pub fn spiro2(a: f64, b: f64) -> DVec2 {
    const N: i32 = 32;
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



pub fn fresnel(s: f64) -> DVec2 {
    let s2 = s * s;
    let theta = s2 / 8.0;
    let tangent = DVec2::from_angle(theta);
    s * spiro2(s2 / 2.0, s2).rotate(tangent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresnel_a(s: f64) -> DVec2 {
        s * spiro2(0.0, 4.0*s*s)
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
    fn test_fresnels() {
        for i in 1..100 {
            let t: f64 = 0.1 * (i as f64);

            let from_a = fresnel_a(t);
            let from_b = fresnel(t);
            let err = (from_a.length() - from_b.length()).abs();

            assert!(err < 0.0001, "t={}: got {} and {}", t, from_a, from_b);
        }
    }
}