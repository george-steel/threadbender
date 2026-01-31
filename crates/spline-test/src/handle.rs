use glam::{DAffine2, DVec2, UVec2};

#[derive(Debug, Clone, Copy)]
pub struct ViewportWindow {
    pub center: DVec2,
    pub scale_factor: f64, // units per pixel
    pub viewport_dims: UVec2,
}

impl ViewportWindow {
    pub fn as_rect(&self) -> (DVec2, DVec2) {
        let radii = (self.scale_factor * 0.5) * self.viewport_dims.as_dvec2();
        (self.center - radii, self.center + radii)
    }

    pub fn to_clip(&self) -> DAffine2 {
        let scales = (2.0 / self.scale_factor) / self.viewport_dims.as_dvec2();
        DAffine2::from_scale(scales) * DAffine2::from_translation(-self.center)
    }
}

#[cfg(test)]
mod tests {
    use glam::{dvec2, uvec2};

    use super::*;

    #[test]
    fn test_viewport_window_clip() {
        let viewport = ViewportWindow {
            center: dvec2(50.0, 50.0),
            scale_factor: 0.5,
            viewport_dims: uvec2(100, 100)
        };
        let (sw, ne) = viewport.as_rect();
        assert_eq!(sw, dvec2(25.0, 25.0));
        assert_eq!(ne, dvec2(75.0, 75.0));

        let mat = viewport.to_clip();
        assert_eq!(mat.transform_point2(sw), dvec2(-1.0, -1.0));
        assert_eq!(mat.transform_point2(ne), dvec2(1.0, 1.0));
    }
}
