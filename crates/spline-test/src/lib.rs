use std::{collections::VecDeque, io::Cursor, sync::Arc};

use glam::{DVec2, UVec2, dvec2, ivec2};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlElement, PointerEvent, window};
use wgpu::Surface;
use leptos::{html::P, prelude::*};
use crate::{display::{GriddedDisplay, SplineEditConnection}, gputil::GPUContext, line::SplineEditState, renderer::{GridParams, LineEditRenderer, RGBA16f}, viewport::{ViewportScroller, ViewportWindow, WorldMouseEvent}};


pub mod gputil;
pub mod util;
mod shaders;
mod viewport;
mod pointer;
mod renderer;
mod display;
mod line;


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

    let true_line = ArcRwSignal::new(Vec::new());

    let edit_state = SplineEditState::new(true_line.clone());
    let editing = Signal::stored(Some(edit_state.make_conn()));

    view!{
        <GriddedDisplay
            grid_params=grid_params.into()
            editing=editing
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