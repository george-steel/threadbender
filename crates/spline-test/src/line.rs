use std::{cmp::max, num, sync::Arc};

use clone_all::clone_all;
use clothoid::spline::ClothoidSplineCage;
use glam::{DVec2, Vec2};
use leptos::prelude::*;

use crate::{display::SplineEditConnection, pointer::MouseButton, renderer::RGBA16f, viewport::{ViewportWindow, WorldMouseEvent}};

#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct DisplayHandle {
    pub world_pos: Vec2,
    pub radius: f32,
    pub pad0: u32,
    pub fill_color: RGBA16f,
    pub line_color: RGBA16f,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtendEnd {
    Start,
    End,
}

#[derive(Clone, Copy, Debug)]
pub enum SplineEditMode {
    Refine,
    Dragging(usize),
    Extend(ExtendEnd),
}

#[derive(Debug, Clone)]
pub struct SplineEditState {
    pub closed_only: bool,
    pub true_line: ArcRwSignal<ClothoidSplineCage>,
    pub mode: ArcRwSignal<SplineEditMode>,
    pub held_point: ArcRwSignal<Option<DVec2>>,
    pub hover_index: ArcRwSignal<Option<usize>>,
}

impl SplineEditState {
    pub const HANDLE_RADIUS: f32 = 10.0; // CSS pixels
    pub const HANDLE_HIT_RADIUS: f64 = 15.0;

    pub const OUTLINE_COLOR: RGBA16f = RGBA16f::rgba(1.0, 1.0, 1.0, 1.0);
    pub const FILL_COLOR_NORMAL: RGBA16f = RGBA16f::rgba(0.1, 0.1, 0.1, 1.0);
    pub const FILL_COLOR_HOVER: RGBA16f = RGBA16f::rgba(0.8, 0.8, 0.8, 1.0);

    pub fn new(true_line: ArcRwSignal<ClothoidSplineCage>, closed_only: bool) -> Self {
        let start_len = true_line.read_untracked().num_points();
        let start_mode = if start_len < 2 {SplineEditMode::Extend(ExtendEnd::End)} else {SplineEditMode::Refine};
        
        SplineEditState {
            closed_only,
            true_line,
            mode: ArcRwSignal::new(start_mode),
            held_point: ArcRwSignal::new(None),
            hover_index: ArcRwSignal::new(None),
        }
    }

    fn make_held_line(self) -> ArcMemo<ClothoidSplineCage> {
        ArcMemo::new_owning(move |old: Option<ClothoidSplineCage>| {
            match self.mode.get() {
                SplineEditMode::Refine => {
                    //let old_empty = if let Some(old_vec) = old {old_vec.is_empty()} else {false};
                    //(Vec::new(), !old_empty)
                    (self.true_line.get(), true)
                },
                SplineEditMode::Dragging(idx) => {
                    let mut line = self.true_line.get();
                    if let Some(p) = self.held_point.get() {
                        line.points[idx] = p;
                    }
                    (line, true)
                },
                SplineEditMode::Extend(end) => {
                    let mut line = self.true_line.get();
                    if let Some(hover) = self.hover_index.get() {
                        if (end == ExtendEnd::Start && hover == line.num_points() - 1) || (end == ExtendEnd::End && hover == 0) {
                            line.closed = true;
                        }
                    } else if let Some(p) = self.held_point.get() {
                        line.closed = false;
                        match end {
                            ExtendEnd::Start => {
                                line.insert_point(0, p, false);
                            },
                            ExtendEnd::End => {
                                line.extend(p, false);
                            },
                        }
                    }
                    (line, true)
                },
            }
        })
    }

    fn make_handles(self) -> ArcMemo<Vec<DisplayHandle>> {
        ArcMemo::new(move |_| {
            let mode = self.mode.get();
            let mut line = self.true_line.get();
            let mut hilight = None;
            let mut radius = Self::HANDLE_RADIUS;

            match mode {
                SplineEditMode::Refine => {
                    hilight = self.hover_index.get();
                },
                SplineEditMode::Dragging(idx) => {
                    if let Some(p) = self.held_point.get() {
                        line.points[idx] = p;
                    }
                    hilight = Some(idx);
                },
                SplineEditMode::Extend(ExtendEnd::Start) => {
                    hilight = Some(0);
                },
                SplineEditMode::Extend(ExtendEnd::End) => {
                    hilight = line.num_points().checked_sub(1);
                    radius = Self::HANDLE_RADIUS as f32 / 2.0; 
                },
            }
            line.points.iter().enumerate().map(|(i, p)| {
                let h = Some(i) == hilight;
                let color = if h {Self::FILL_COLOR_HOVER} else {Self::FILL_COLOR_NORMAL};
                let radius = if h {Self::HANDLE_RADIUS} else {radius};
                DisplayHandle {
                    world_pos: p.as_vec2(),
                    radius,
                    pad0: 0,
                    fill_color: color,
                    line_color: Self::OUTLINE_COLOR,
                }
            }).collect()
        })
    }

    pub fn make_conn(self) -> SplineEditConnection {
        SplineEditConnection {
            handles: self.clone().make_handles().into(),
            line: self.clone().make_held_line().into(),
            on_mouse: Arc::new(move |ev, view| {self.handle_mouse(ev, view);}),
        }
    }

    fn hit_test(&self, hit_point: DVec2, max_dist: f64) -> Option<usize> {
        let mut hit = None;
        let mut dist = max_dist;
        for (i, p) in self.true_line.read_untracked().points.iter().enumerate() {
            let d = p.distance(hit_point);
            if d < dist {
                hit = Some(i);
                dist = d;
            }
        }
        hit
    }

    fn set_refine(&self) {
        self.mode.set(SplineEditMode::Refine);
        self.hover_index.set(None);
        self.held_point.set(None);
    }

    pub fn handle_mouse(&self, ev: WorldMouseEvent, view: &ViewportWindow) {
        let hover_rad = Self::HANDLE_HIT_RADIUS * view.css_px_ratio / view.px_per_unit;
        match ev {
            WorldMouseEvent::Out => {
                self.hover_index.set(None);
                if let SplineEditMode::Extend(_) = self.mode.get_untracked() {
                    self.held_point.set(None);
                }
            },
            WorldMouseEvent::Hover(p) => {
                let hit = self.hit_test(p, hover_rad);
                if let (None, SplineEditMode::Extend(_)) = (hit, self.mode.get_untracked()) {
                    self.hover_index.set(None);
                    self.held_point.set(Some(p));
                } else {
                    let old_hit = self.hover_index.get_untracked();
                    if hit != old_hit {
                        self.held_point.set(None);
                        self.hover_index.set(hit);
                    }
                }
            },
            WorldMouseEvent::Click(button, p) => {
                let hit = self.hit_test(p, hover_rad);
                let num_points = self.true_line.read_untracked().num_points();
                match self.mode.get_untracked() {
                    SplineEditMode::Refine => {
                        if hit == num_points.checked_sub(1) {
                            self.mode.set(SplineEditMode::Extend(ExtendEnd::End));
                        } else if hit == Some(0){
                            self.mode.set(SplineEditMode::Extend(ExtendEnd::Start));
                        }
                    },
                    SplineEditMode::Dragging(_) => {
                        log::warn!("click detected when dragging");
                    },
                    SplineEditMode::Extend(ExtendEnd::Start) => {
                        if num_points >= 2 && hit == Some(0) {
                            self.set_refine();
                        } else if num_points >= 2 && hit == Some(num_points - 1) {
                            self.true_line.write().closed = true;
                            self.set_refine();
                        } else {
                            self.true_line.write().insert_point(0, p, button == MouseButton::Right);
                        }
                    },
                    SplineEditMode::Extend(ExtendEnd::End) => {
                        if num_points >= 2 && hit == Some(num_points - 1) {
                            self.set_refine();
                        } else if num_points >= 2 && hit == Some(0) {
                            self.true_line.write().closed = true;
                            self.set_refine();
                        } else {
                            self.true_line.write().extend(p, button == MouseButton::Right);
                        }
                    },
                }
            },
            WorldMouseEvent::DragStart(button, p) => {
                let hit = self.hit_test(p, hover_rad);
                match self.mode.get_untracked() {
                    SplineEditMode::Refine => {
                        if let Some(i) = hit {
                            self.mode.set(SplineEditMode::Dragging(i));
                            self.held_point.set(Some(p));
                        }
                    },
                    SplineEditMode::Dragging(_) => {
                        log::warn!("nested drags detected");
                    },
                    SplineEditMode::Extend(_) => {
                        if let Some(i) = hit {
                            self.mode.set(SplineEditMode::Dragging(i));
                        }
                        self.held_point.set(Some(p));
                    },
                }
            },
            WorldMouseEvent::DragMove(p) => {match self.mode.get_untracked() {
                SplineEditMode::Refine => {},
                SplineEditMode::Dragging(_) | SplineEditMode::Extend(_) => {
                    self.held_point.set(Some(p));
                },
            }},
            WorldMouseEvent::DragDone(button, p) => {match self.mode.get_untracked() {
                SplineEditMode::Refine => {},
                SplineEditMode::Dragging(idx) => {
                    self.true_line.write().points[idx] = p;
                    self.set_refine();
                },
                SplineEditMode::Extend(ExtendEnd::Start) => {
                    self.true_line.write().insert_point(0, p, button == MouseButton::Right);
                    self.held_point.set(None);
                },
                SplineEditMode::Extend(ExtendEnd::End) => {
                    self.true_line.write().extend(p, button == MouseButton::Right);
                    self.held_point.set(None);
                },

            }},
            WorldMouseEvent::DragCancel => {match self.mode.get_untracked() {
                SplineEditMode::Refine => {},
                SplineEditMode::Dragging(_) => {
                    self.set_refine();
                },
                SplineEditMode::Extend(_) => {
                    self.held_point.set(None);
                },
            }},
        }
    }
}