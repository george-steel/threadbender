use glam::{UVec2, dvec2, uvec2};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlElement, PointerEvent, window};
use wgpu::Surface;
use crate::{gputil::GPUContext, grid::{GridParams, GridUniforms, GriddedRenderer, RGBA16f}, viewport::ViewportScroller};


pub mod gputil;
mod shaders;
mod viewport;
mod pointer;
mod grid;

#[wasm_bindgen]
pub struct PointerTest {
    log: HtmlElement,
    gestures: pointer::GestureRecognizer,
    scroller: ViewportScroller,
    gpu: GPUContext,
    renderer: GriddedRenderer,
    grid_params: GridParams,
}

#[wasm_bindgen]
impl PointerTest {
    pub async fn init(canvas: HtmlCanvasElement, log: HtmlElement) -> Self {
        let css_width = canvas.client_width();
        let css_height =  canvas.client_height();
        let gestures = pointer::GestureRecognizer::new(dvec2(css_width as f64, css_height as f64));
        let init_dev_size = uvec2(css_width as u32, css_height as u32);
        let init_rad = dvec2(10.0, 10.0);
        let scroller = ViewportScroller::new(init_dev_size, init_rad);

        let wgpu_inst = wgpu::Instance::default();
        let surface = wgpu_inst.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).unwrap();
        let gpu = GPUContext::with_limits(
            wgpu_inst,
            Some(&surface),
            wgpu::Features::RG11B10UFLOAT_RENDERABLE,
            Default::default(),
        ).await;
        log::info!("initialized gpu");
        
        let canvas_size = uvec2(canvas.width(), canvas.height());
        let renderer = GriddedRenderer::new(&gpu, surface, canvas_size);
        let grid_params = GridParams {
            line_spacing: 1.0,
            major_every: 5,
            line_color: RGBA16f::rgba(0.0, 0.0, 0.0, 0.1),
            major_color: RGBA16f::rgba(0.0, 0.0, 0.0, 0.3),
            axis_color: RGBA16f::rgba(0.0, 0.0, 0.0, 1.0),
            background_color: RGBA16f::rgba(0.0, 1.0, 1.0, 1.0),
        };
        PointerTest {
            log,
            gestures, scroller,
            gpu, renderer,
            grid_params,
        }
    }

    pub fn add_log(&self, message: &str) {
        let doc = window().unwrap().document().unwrap();
        let entry = doc.create_element("p").unwrap().dyn_into::<HtmlElement>().unwrap();
        entry.set_inner_text(&message);
        self.log.append_child(&entry);
        entry.scroll_into_view();
    }

    pub fn on_pointer_event(&mut self, raw_event: PointerEvent) -> bool{
        let cooked_events = self.gestures.process_event(&raw_event);
        let mut anydirty = false;
        for g in cooked_events {
            let (opt_ev, dirty) = self.scroller.handle_gesture(g);
            anydirty = anydirty || dirty;
            if let Some(e) = opt_ev {
                self.add_log(&format!("{:?}, {:?}", e, self.scroller.current_view.center));
            }
        }
        anydirty
    }

    pub fn on_resize(&mut self, css_width: f64, css_height: f64, device_width: u32, device_height: u32) {
        let css_size = dvec2(css_width, css_height);
        let device_size = uvec2(device_width, device_height);
        let gestures = self.gestures.resize(css_size);
        for g in gestures {
            let (opt_ev, dirty) = self.scroller.handle_gesture(g);
            if let Some(e) = opt_ev {
                self.add_log(&format!("{:?}", e));
            }
        }
        self.scroller.handle_resize(device_size);
        self.renderer.resize(&self.gpu, device_size);
    }

    pub fn on_frame(&mut self) {
        let grid_uniforms = &GridUniforms {
            viewport: self.scroller.current_view.to_uniforms(),
            params: self.grid_params
        };
        let res = self.renderer.render(&self.gpu, grid_uniforms, move |_,_| {});
        if let Err(e) = res {
            let message = format!("Error with swapchain: {}", e);
            log::warn!("{}", &message);
            self.add_log(&message);
        }
    }
}

#[wasm_bindgen(start)]
pub fn init_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug);
    log::info!("Hello from wasm");
    Ok(())
}