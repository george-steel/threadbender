use std::{collections::VecDeque, io::Cursor, sync::Arc};

use glam::{UVec2, dvec2, uvec2};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlElement, PointerEvent, window};
use wgpu::Surface;
use leptos::{html::P, prelude::*};
use crate::{gputil::GPUContext, grid::{GridParams, GridUniforms, GriddedRenderer, RGBA16f}, viewport::{ViewportScroller, WorldMouseEvent}, display::GriddedDisplay};


pub mod gputil;
pub mod util;
mod shaders;
mod viewport;
mod pointer;
mod grid;
mod display;

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
        line_color: RGBA16f::rgba(0.0, 0.0, 0.0, 0.1),
        major_color: RGBA16f::rgba(0.0, 0.0, 0.0, 0.3),
        axis_color: RGBA16f::rgba(0.0, 0.0, 0.0, 1.0),
        background_color: RGBA16f::rgba(0.0, 1.0, 1.0, 1.0),
    });

    let messages = RwSignal::new(VecDeque::<String>::new());
    let on_mouse = move |ev:WorldMouseEvent| {
        let msg = format!("{:?}", ev);
        messages.update(|q| {
            q.push_back(msg);
            if q.len() > 50 {
                q.pop_front();
            }
        });
    };

    view!{
        <GriddedDisplay
            grid_params=grid_params.into()
            on_mouse=on_mouse
        />
        <div id="event_log">
            <For
            each=move ||{messages.get()}
            key=String::clone
            children = GrabbingP
            />
        </div>
    }
}

#[wasm_bindgen]
pub fn main() {
    mount_to_body(App);
}

#[wasm_bindgen(start)]
pub fn init_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug);
    log::info!("Hello from wasm");
    Ok(())
}