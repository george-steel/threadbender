use std::{collections::VecDeque, io::Cursor, sync::Arc};

use glam::{DVec2, UVec2, dvec2, ivec2};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlElement, PointerEvent, window};
use wgpu::Surface;
use leptos::{html::P, prelude::*};
use crate::{display::GriddedDisplay, gputil::GPUContext, line::LineEditState, renderer::{GridParams, LineEditRenderer, RGBA16f}, viewport::{ViewportScroller, ViewportWindow, WorldMouseEvent}};


pub mod gputil;
pub mod util;
mod shaders;
mod viewport;
mod pointer;
mod renderer;
mod display;
mod line;

fn GrabbingP(message: String) -> impl IntoView {
    let p_ref = NodeRef::<P>::new();
    
    p_ref.on_load(move |p|{
        p.scroll_into_view();
    });

    view!{
        <p node_ref=p_ref>{message}</p>
    }
}


#[component]
fn App() -> impl IntoView {
    let grid_params = RwSignal::new(GridParams {
        line_spacing: 1.0,
        major_every: 5,
        line_color: RGBA16f::rgba(1.0, 1.0, 1.0, 0.1),
        major_color: RGBA16f::rgba(1.0, 1.0, 1.0, 0.3),
        axis_color: RGBA16f::rgba(1.0, 1.0, 1.0, 1.0),
        background_color: RGBA16f::rgba(0.0, 0.0, 0.0, 0.0),
    });

    let init_line: Vec<DVec2> = (-5..5).map(|x: i32| {ivec2(x, x).as_dvec2()}).collect();
    let true_line = ArcRwSignal::new(init_line);

    let edit_state = StoredValue::new(LineEditState::new(true_line.clone()));
    let on_mouse = move |ev:WorldMouseEvent, view: &ViewportWindow| {
        if let Some(ref mut st) = edit_state.try_write_value() {
            st.handle_mouse(&ev, view);
        } else {
            log::error!("edit_state is disposed");
        }
    };

    let handle_sig = edit_state.read_value().handles.clone();
    let held_line = edit_state.read_value().held_line.clone();

    view!{
        <GriddedDisplay
            grid_params=grid_params.into()
            handles=handle_sig.into()
            line=held_line.into()
            on_mouse=on_mouse
        />
    }
}

#[wasm_bindgen]
pub fn run_app() {
    mount_to_body(App);
}

#[wasm_bindgen(start)]
pub fn init_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug);
    log::info!("Hello from wasm");
    Ok(())
}