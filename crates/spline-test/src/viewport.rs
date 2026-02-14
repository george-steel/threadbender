use glam::{DAffine2, DVec2, UVec2, Vec2};

use crate::pointer::GestureEvent;

#[derive(Debug, Clone, Copy)]
pub struct ViewportWindow {
    pub center: DVec2,
    pub px_per_unit: f64,
    pub viewport_dims: UVec2,
}

impl ViewportWindow {
    pub fn as_rect(&self) -> (DVec2, DVec2) {
        let radii = (0.5 / self.px_per_unit) * self.viewport_dims.as_dvec2();
        (self.center - radii, self.center + radii)
    }

    pub fn to_clip(&self) -> DAffine2 {
        let scales = (2.0 * self.px_per_unit) / self.viewport_dims.as_dvec2();
        DAffine2::from_scale(scales) * DAffine2::from_translation(-self.center)
    }

    pub fn scrolled(&self, zoom: f64, trans: DVec2) -> Self {
        let new_zoom = zoom * self.px_per_unit;
        let new_center = self.center - trans * (0.5 / new_zoom) * self.viewport_dims.as_dvec2();
        ViewportWindow {
            center: new_center,
            px_per_unit: new_zoom,
            viewport_dims: self.viewport_dims,
        }
    }

    pub fn resized(&self, new_size: UVec2) -> Self {
        ViewportWindow {
            center: self.center,
            px_per_unit: self.px_per_unit,
            viewport_dims: new_size,
        }
    }

    pub fn to_uniforms(&self) -> ViewportUniforms {
        let scales = ((2.0 * self.px_per_unit) / self.viewport_dims.as_dvec2()).as_vec2();
        let trans = - scales * self.center.as_vec2();
        let (sw, ne) = self.as_rect();
        ViewportUniforms { scales, trans, sw: sw.as_vec2(), ne: ne.as_vec2() }
    }
}

// GPU structure
#[derive(Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C, align(8))]
pub struct ViewportUniforms {
    pub scales: Vec2,
    pub trans: Vec2,
    pub sw: Vec2,
    pub ne: Vec2,
}

impl ViewportUniforms {
    pub fn transform_point(&self, p: Vec2) -> Vec2 {
        self.scales * p + self.trans
    }
}

// All values in world space
#[derive(Clone, Copy, Debug)]
pub enum WorldMouseEvent {
    Out,
    Hover(DVec2),
    Click(DVec2),
    DragStart(DVec2), // hit location
    DragMove(DVec2), // current location
    DragDone(DVec2),
    DragCancel,
}

#[derive(Clone, Debug)]
pub struct ViewportScroller {
    pub current_view: ViewportWindow,
    pub scroll_start: Option<ViewportWindow>,
}

impl ViewportScroller {
    pub fn new(device_size: UVec2, show_radius: DVec2) -> Self {
        let zoom = (device_size.as_dvec2() / (show_radius * 2.0)).min_element();
        let view = ViewportWindow {
            center: DVec2::ZERO,
            px_per_unit: zoom,
            viewport_dims: device_size,
        };
        Self {
            current_view: view,
            scroll_start: None,
        }
    }

    pub fn handle_gesture(&mut self, event: GestureEvent) -> (Option<WorldMouseEvent>, bool) {
        let from_clip = self.current_view.to_clip().inverse();
        match event {
            GestureEvent::Out => {
                (Some(WorldMouseEvent::Out), false)
            },
            GestureEvent::Hover(clip) => {
                let from_clip = self.current_view.to_clip().inverse();
                let world = from_clip.transform_point2(clip);
                (Some(WorldMouseEvent::Hover(world)), false)
            },
            GestureEvent::Click(clip) => {
                let from_clip = self.current_view.to_clip().inverse();
                let world = from_clip.transform_point2(clip);
                (Some(WorldMouseEvent::Click(world)), false)
            },
            GestureEvent::DragStart(clip) => {
                let from_clip = self.current_view.to_clip().inverse();
                let world = from_clip.transform_point2(clip);
                (Some(WorldMouseEvent::DragStart(world)), false)
            },
            GestureEvent::DragMove(clip) => {
                let from_clip = self.current_view.to_clip().inverse();
                let world = from_clip.transform_point2(clip);
                (Some(WorldMouseEvent::DragMove(world)), false)
            },
            GestureEvent::DragDone(clip) => {
                let from_clip = self.current_view.to_clip().inverse();
                let world = from_clip.transform_point2(clip);
                (Some(WorldMouseEvent::DragDone(world)), false)
            },
            GestureEvent::DragCancel => {
                (Some(WorldMouseEvent::DragCancel), false)
            },
            GestureEvent::ScrollStart => {
                self.scroll_start = Some(self.current_view);
                (None, false)
            },
            GestureEvent::ScrollMove(zoom, trans) => {
                if let Some(start) = self.scroll_start {
                    self.current_view = start.scrolled(zoom, trans);
                    (None, true)
                } else {
                    (None, false)
                }
            },
            GestureEvent::ScrollDone => {
                self.scroll_start = None;
                (None, false)
            },
        }
    }

    pub fn handle_resize(&mut self, device_size: UVec2) {
        self.current_view = self.current_view.resized(device_size);
        self.scroll_start = None;
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
            px_per_unit: 2.0,
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
