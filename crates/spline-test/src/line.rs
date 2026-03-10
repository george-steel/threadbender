use std::cmp::max;

use clone_all::clone_all;
use glam::{DVec2, Vec2};
use leptos::prelude::*;

use crate::{renderer::RGBA16f, viewport::{ViewportWindow, WorldMouseEvent}};

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct DisplayHandle {
    pub world_pos: Vec2,
    pub radius: f32,
    pub pad0: u32,
    pub fill_color: RGBA16f,
    pub line_color: RGBA16f,
}

#[derive(Debug, Clone)]
pub struct LineEditState {
    pub true_line: ArcRwSignal<Vec<DVec2>>,
    pub held_line: ArcRwSignal<Vec<DVec2>>,
    pub drag_index: ArcRwSignal<Option<usize>>,
    pub hover_index: ArcRwSignal<Option<usize>>,
    pub handles: ArcMemo<Vec<DisplayHandle>>,
}

impl LineEditState {
    pub const HANDLE_RADIUS: f64 = 10.0; // CSS pixels
    pub const HANDLE_HIT_RADIUS: f64 = 15.0;

    pub const OUTLINE_COLOR: RGBA16f = RGBA16f::rgba(1.0, 1.0, 1.0, 1.0);
    pub const FILL_COLOR_NORMAL: RGBA16f = RGBA16f::rgba(0.1, 0.1, 0.1, 1.0);
    pub const FILL_COLOR_HOVER: RGBA16f = RGBA16f::rgba(0.8, 0.8, 0.8, 1.0);

    pub fn new(true_line: ArcRwSignal<Vec<DVec2>>) -> Self {
        let held_line = ArcRwSignal::new(true_line.get_untracked());
        let drag_index = ArcRwSignal::new(None);
        let hover_index = ArcRwSignal::new(None);

        let handles = ArcMemo::new_owning({clone_all!(held_line, drag_index, hover_index); move |_| {
            log::debug!("running handles memo");
            let dragging = drag_index.get();
            let hovering = hover_index.get();
            let held = held_line.read();

            let handles: Vec<DisplayHandle> = held.iter().enumerate().map(|(i, p)| {
                let hover = (Some(i) == dragging) || (Some(i) == hovering);
                let color = if hover {Self::FILL_COLOR_HOVER} else {Self::FILL_COLOR_NORMAL};
                DisplayHandle {
                    world_pos: p.as_vec2(),
                    radius: Self::HANDLE_RADIUS as f32,
                    pad0: 0,
                    fill_color: color,
                    line_color: Self::OUTLINE_COLOR,
                }
            }).collect();
            (handles, true)
        }});

        LineEditState {
            true_line,
            held_line,
            drag_index,
            hover_index,
            handles
        }
    }

    fn hit_test(&self, hit_point: DVec2, max_dist: f64) -> Option<usize> {
        let mut hit = None;
        let mut dist = max_dist;
        for (i, p) in self.held_line.read_untracked().iter().enumerate() {
            let d = p.distance(hit_point);
            if d < dist {
                hit = Some(i);
                dist = d;
            }
        }
        hit
    }

    pub fn handle_mouse(&mut self, ev: &WorldMouseEvent, view: &ViewportWindow) {
        match ev {
            WorldMouseEvent::Out => {},
            WorldMouseEvent::Hover(p) => {
                let hover_rad = Self::HANDLE_RADIUS * view.css_px_ratio / view.px_per_unit;
                let hit = self.hit_test(*p, hover_rad);
                let old_hit = self.hover_index.get_untracked();
                if hit != old_hit {
                    self.hover_index.set(hit);
                }
            },
            WorldMouseEvent::Click(p) => {},
            WorldMouseEvent::DragStart(p) => {
                log::info!("DragStart {}", *p);
                let hover_rad = Self::HANDLE_RADIUS * view.css_px_ratio / view.px_per_unit;
                let hit = self.hit_test(*p, hover_rad);
                log::info!("hit {:?} radius {} units", hit, hover_rad);
                self.drag_index.set(hit);
                self.hover_index.set(None);
            },
            WorldMouseEvent::DragMove(p) => {
                log::info!("DragMove");
                if let Some(i) = self.drag_index.get_untracked() {
                    log::info!("drag_index {}", i);
                    self.held_line.write()[i] = *p;
                } 
            },
            WorldMouseEvent::DragDone(p) => {
                if let Some(i) = self.drag_index.get_untracked() {
                    self.held_line.write()[i] = *p;
                    self.true_line.write()[i] = *p;
                }
                self.drag_index.set(None);
            },
            WorldMouseEvent::DragCancel => {
                if let Some(i) = self.drag_index.get_untracked() {
                    self.held_line.write()[i] = self.true_line.read_untracked()[i];
                }
            },
        }
    }
}