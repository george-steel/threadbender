use std::collections::HashSet;

use glam::{DVec2, dvec2};
use web_sys::js_sys;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PointerID(i32);

#[derive(Clone, Copy, Debug)]
pub enum GestureEvent {
    Out,
    Hover(DVec2), // clip space
    Click(DVec2),
    DragStart(DVec2), // hit location
    DragMove(DVec2), // current location
    DragDone(DVec2),
    DragCancel,
    ScrollStart,
    ScrollMove(f64, DVec2), // relative to scroll start, acts on old clip space
    ScrollDone,
}

#[derive(Clone, Debug)]
enum GestureState{
    Idle,
    Down(PointerID, DVec2), // start position in clip space
    Dragging(PointerID, DVec2),
    TouchScroll(TouchScrollState, TouchScrollState),
    MouseScroll(PointerID, f64, DVec2),
    WaitForSettle(HashSet<PointerID>),
}

#[derive(Copy, Clone, Debug)]
struct TouchScrollState {
    pub ptr: PointerID,
    pub started: DVec2,
    pub current: DVec2,
}

pub struct GestureRecognizer {
    css_size: DVec2,
    pointers_in: HashSet<PointerID>,
    state: GestureState,
}

impl GestureRecognizer {
    const MIN_DRAG_DIST: f64 = 4.0;

    pub fn new(size: DVec2) -> Self {
        GestureRecognizer {
            css_size: size,
            pointers_in: HashSet::new(),
            state: GestureState::Idle
        }
    }

    pub fn resize(&mut self, new_size: DVec2) -> Vec<GestureEvent> {
        self.css_size = new_size;
        match self.state {
            GestureState::Idle => {
                vec![GestureEvent::Out]
            },
            GestureState::Down(pointer, _) => {
                self.state = GestureState::WaitForSettle(HashSet::from([pointer]));
                vec![GestureEvent::Out]
            },
            GestureState::Dragging(pointer, _) => {
                self.state = GestureState::WaitForSettle(HashSet::from([pointer]));
                vec![GestureEvent::DragCancel]
            },
            GestureState::TouchScroll(a, b) => {
                self.state = GestureState::WaitForSettle(HashSet::from([a.ptr, b.ptr]));
                vec![GestureEvent::ScrollDone]
            },
            GestureState::MouseScroll(pointer, _, _) => {
                self.state = GestureState::WaitForSettle(HashSet::from([pointer]));
                vec![GestureEvent::ScrollDone]
            },
            GestureState::WaitForSettle(_) => {
                Vec::new()
            },
        }
    }

    fn to_clip(&self, pos: DVec2) -> DVec2 {
        (dvec2(2.0, -2.0) * pos / self.css_size) + dvec2(-1.0, 1.0)
    }

    fn get_touch_scroll(&self, a: TouchScrollState, b: TouchScrollState) -> (f64, DVec2) {
        let old_centroid = self.to_clip(0.5 * (a.started + b.started));
        let new_centroid = self.to_clip(0.5 * (a.current + b.current));
        let zoom = (a.current.distance(b.current) / a.started.distance(b.started)).clamp(0.1, 10.0);
        let trans = new_centroid - zoom * old_centroid;
        (zoom, trans)
    }

    pub fn process_event(&mut self, event: &web_sys::PointerEvent) -> Vec<GestureEvent> {
        let etype = event.type_();
        let ptr = PointerID(event.pointer_id());
        let offset_x = js_sys::Reflect::get(&event, &"offsetX".into()).unwrap().as_f64().unwrap();
        let offset_y = js_sys::Reflect::get(&event, &"offsetY".into()).unwrap().as_f64().unwrap();
        let offset = dvec2(offset_x, offset_y);
        let clip = self.to_clip(offset);
        if etype == "pointermove" { match self.state {
            GestureState::Idle => {
                if self.pointers_in.len() == 1 && self.pointers_in.contains(&ptr) {
                    vec![GestureEvent::Hover(clip)]
                } else {
                    Vec::new()
                }
            },
            GestureState::Down(pressed, started) => {
                if ptr == pressed && offset.distance(started) >= Self::MIN_DRAG_DIST {
                    self.state = GestureState::Dragging(pressed, started);
                    let start_clip = (dvec2(3.0, -2.0) * started / self.css_size) + dvec2(-1.0, 1.0);
                    vec![GestureEvent::DragStart(start_clip), GestureEvent::DragMove(clip)]
                } else {
                    Vec::new()
                }
            },
            GestureState::Dragging(pressed, started) => {
                if ptr == pressed {
                    vec![GestureEvent::DragMove(clip)]
                } else {
                    Vec::new()
                }
            },
            GestureState::TouchScroll(ref mut a, ref mut b) => {
                if ptr == a.ptr {
                    a.current = offset;
                } else if ptr == b.ptr {
                    b.current = offset;
                }
                let aa = *a;
                let bb = *b;
                let (zoom, trans) = self.get_touch_scroll(aa, bb);
                vec![GestureEvent::ScrollMove(zoom, trans)]
            },
            GestureState::MouseScroll(pressed, zoom, started) => {
                if ptr == pressed {
                    let old = self.to_clip(started);
                    let trans = clip * zoom - old;
                    vec![GestureEvent::ScrollMove(zoom, trans)]
                } else {
                    Vec::new()
                }
            },
            GestureState::WaitForSettle(_) => {
                Vec::new()
            },
        }} else if etype == "pointerup" { match self.state {
            GestureState::Idle => {
                log::warn!("pointerup fired with no pressed pointers tracked");
                Vec::new()
            },
            GestureState::Down(pressed, started) => {
                if ptr == pressed {
                    self.state = GestureState::Idle;
                    let mut out = vec![GestureEvent::Click(self.to_clip(started))];
                    if self.pointers_in.len() == 1 && self.pointers_in.contains(&ptr) {
                        out.push(GestureEvent::Hover(clip));
                    }
                    out
                } else {
                    log::warn!("pointerup fired on untracked pointer");
                    Vec::new()
                }
            },
            GestureState::Dragging(pressed, _) => {
                if ptr == pressed {
                    self.state = GestureState::Idle;
                    let mut out = vec![GestureEvent::DragDone(clip)];
                    if self.pointers_in.len() == 1 && self.pointers_in.contains(&ptr) {
                        out.push(GestureEvent::Hover(clip));
                    }
                    out
                } else {
                    log::warn!("pointerup fired on untracked pointer");
                    Vec::new()
                }
            },
            GestureState::TouchScroll(a, b) => {
                if ptr == a.ptr || ptr == b.ptr {
                    let mut down = HashSet::from([a.ptr, b.ptr]);
                    down.remove(&ptr);
                    self.state = GestureState::WaitForSettle(down);
                    vec![GestureEvent::ScrollDone]
                } else {
                    log::warn!("pointerup fired on untracked pointer");
                    Vec::new()
                }
            },
            GestureState::MouseScroll(pressed, _, _) => {
                if ptr == pressed {
                    self.state = GestureState::Idle;
                    let mut out = vec![GestureEvent::ScrollDone];
                    if self.pointers_in.len() == 1 && self.pointers_in.contains(&ptr) {
                        out.push(GestureEvent::Hover(clip));
                    }
                    out
                } else {
                    log::warn!("pointerup fired on untracked pointer");
                    Vec::new()
                }
            },
            GestureState::WaitForSettle(ref mut pressed) => {
                pressed.remove(&ptr);
                if pressed.is_empty() {
                    self.state = GestureState::Idle;
                } 
                Vec::new()
            },
        }} else if etype == "pointerdown" { match self.state {
            GestureState::Idle => {
                let buttons = event.buttons();
                if buttons == 1 {
                    self.state = GestureState::Down(ptr, offset);
                    vec![GestureEvent::Hover(clip)]
                } else if buttons == 4 {
                    self.state = GestureState::MouseScroll(ptr, 1.0, offset);
                    vec![GestureEvent::Out, GestureEvent::ScrollStart]
                } else {
                    let down = HashSet::from([ptr]);
                    self.state = GestureState::WaitForSettle(down);
                    vec![GestureEvent::Out]
                }
            },
            GestureState::Down(pressed, started) => {
                if ptr != pressed {
                    let a = TouchScrollState{ptr: pressed, started, current: started};
                    let b = TouchScrollState{ptr, started: offset, current: offset};
                    self.state = GestureState::TouchScroll(a, b);
                    vec![GestureEvent::Out, GestureEvent::ScrollStart]
                } else {
                    log::warn!("duplicate pointerdown");
                    Vec::new()
                }
            },
            GestureState::Dragging(pressed, _) => {
                if ptr != pressed {
                    let down = HashSet::from([pressed, ptr]);
                    self.state = GestureState::WaitForSettle(down);
                    vec![GestureEvent::Out, GestureEvent::DragCancel]
                } else {
                    log::warn!("duplicate pointerdown");
                    Vec::new()
                }
            },
            GestureState::TouchScroll(a, b) => {
                if ptr != a.ptr && ptr != b.ptr {
                    let down = HashSet::from([a.ptr, b.ptr, ptr]);
                    self.state = GestureState::WaitForSettle(down);
                    vec![GestureEvent::ScrollDone]
                } else {
                    log::warn!("duplicate pointerdown");
                    Vec::new()
                }
            },
            GestureState::MouseScroll(pressed, _, _) => {
                if ptr != pressed {
                    let down = HashSet::from([pressed, ptr]);
                    self.state = GestureState::WaitForSettle(down);
                    vec![GestureEvent::ScrollDone]
                } else {
                    log::warn!("duplicate pointerdown");
                    Vec::new()
                }
            },
            GestureState::WaitForSettle(ref mut down) => {
                down.insert(ptr);
                Vec::new()
            },
        }} else if etype == "pointercancel" { match self.state {
            GestureState::Idle => {
                log::warn!("pointercancel fired with no pressed pointers tracked");
                Vec::new()
            },
            GestureState::Down(pressed, started) => {
                if ptr == pressed {
                    self.state = GestureState::Idle;
                    if self.pointers_in.len() == 1 && self.pointers_in.contains(&ptr) {
                        vec![GestureEvent::Hover(clip)]
                    } else {
                        Vec::new()
                    }
                } else {
                    log::warn!("pointercancel fired on untracked pointer");
                    Vec::new()
                }
            },
            GestureState::Dragging(pressed, _) => {
                if ptr == pressed {
                    self.state = GestureState::Idle;
                    let mut out = vec![GestureEvent::DragCancel];
                    if self.pointers_in.len() == 1 && self.pointers_in.contains(&ptr) {
                        out.push(GestureEvent::Hover(clip));
                    }
                    out
                } else {
                    log::warn!("pointercancel fired on untracked pointer");
                    Vec::new()
                }
            },
            GestureState::TouchScroll(a, b) => {
                if ptr == a.ptr || ptr == b.ptr {
                    let mut down = HashSet::from([a.ptr, b.ptr]);
                    down.remove(&ptr);
                    self.state = GestureState::WaitForSettle(down);
                    vec![GestureEvent::ScrollMove(1.0, DVec2::ZERO), GestureEvent::ScrollDone]
                } else {
                    log::warn!("pointercancel fired on untracked pointer");
                    Vec::new()
                }
            },
            GestureState::MouseScroll(pressed, _, _) => {
                if ptr == pressed {
                    self.state = GestureState::Idle;
                    let mut out = vec![GestureEvent::ScrollMove(1.0, DVec2::ZERO), GestureEvent::ScrollDone];
                    if self.pointers_in.len() == 1 && self.pointers_in.contains(&ptr) {
                        out.push(GestureEvent::Hover(clip));
                    }
                    out
                } else {
                    log::warn!("pointercancel fired on untracked pointer");
                    Vec::new()
                }
            },
            GestureState::WaitForSettle(ref mut pressed) => {
                pressed.remove(&ptr);
                if pressed.is_empty() {
                    self.state = GestureState::Idle;
                } 
                Vec::new()
            },
        }} else if etype == "pointerenter" {
            self.pointers_in.insert(ptr);
            if let GestureState::Idle = self.state {
                if self.pointers_in.len() == 1 {
                    vec![GestureEvent::Hover(clip)]
                } else {
                    vec![GestureEvent::Out]
                }
            } else {
                Vec::new()
            }
        } else if etype == "pointerleave" {
            self.pointers_in.remove(&ptr);
            if let GestureState::Idle = self.state {
                if self.pointers_in.len() == 1 {
                    Vec::new()
                } else {
                    vec![GestureEvent::Out]
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    pub fn process_wheel(&mut self, event: &web_sys::WheelEvent) -> Option<(i32, DVec2)> {
        let offset_x = js_sys::Reflect::get(&event, &"offsetX".into()).unwrap().as_f64().unwrap();
        let offset_y = js_sys::Reflect::get(&event, &"offsetY".into()).unwrap().as_f64().unwrap();
        let offset = dvec2(offset_x, offset_y);
        let clip = self.to_clip(offset);
        let delta = event.delta_y().signum() as i32;
        match self.state {
            GestureState::Idle => Some((delta, clip)),
            _ => None
        }
    }
}