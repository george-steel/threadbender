use glam::{UVec2, dvec2, uvec2};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlElement, PointerEvent, window};
use crate::{gputil::GPUContext, viewport::ViewportScroller};


pub mod gputil;
mod shaders;
mod viewport;
mod pointer;

#[wasm_bindgen]
pub struct Application {
    gpu: GPUContext,
    #[wasm_bindgen(skip)]
    pub surface: wgpu::Surface<'static>,
}

impl Application {
    pub fn new(gpu: GPUContext, surface: wgpu::Surface<'static>, size: UVec2) -> Self {
        Application {
            gpu, surface,
        }
    }
}

#[wasm_bindgen]
impl Application {
    #[cfg(target_arch = "wasm32")]
    pub async fn init_from_canvas(canvas: HtmlCanvasElement) -> Self {
        let init_size = UVec2::new(canvas.width(), canvas.height());
        log::info!("got canvas size");

        let wgpu_inst = wgpu::Instance::default();
        let surface = wgpu_inst.create_surface(wgpu::SurfaceTarget::Canvas(canvas)).unwrap();
        let gpu = GPUContext::with_limits(
            wgpu_inst,
            Some(&surface),
            wgpu::Features::RG11B10UFLOAT_RENDERABLE,
            Default::default(),
        ).await;
        log::info!("initialized gpu");

        gpu.configure_surface_target(&surface, init_size);
        log::info!("configured canvas");

        Self::new(gpu, surface, init_size)
    }

    pub fn render(&self) {

    }
}

#[wasm_bindgen]
pub struct PointerTest {
    canvas: HtmlCanvasElement,
    log: HtmlElement,
    gestures: pointer::GestureRecognizer,
    scroller: ViewportScroller,
}

#[wasm_bindgen]
impl PointerTest {
    pub fn init(canvas: HtmlCanvasElement, log: HtmlElement) -> Self {
        let css_width = canvas.client_width();
        let css_height =  canvas.client_height();
        let gestures = pointer::GestureRecognizer::new(dvec2(css_width as f64, css_height as f64));
        let init_dev_size = uvec2(css_width as u32, css_height as u32);
        let init_rad = dvec2(10.0, 10.0);
        let scroller = ViewportScroller::new(init_dev_size, init_rad);
        PointerTest { canvas, log, gestures, scroller}
    }

    pub fn add_log(&self, message: &str) {
        let doc = window().unwrap().document().unwrap();
        let entry = doc.create_element("p").unwrap().dyn_into::<HtmlElement>().unwrap();
        entry.set_inner_text(&message);
        self.log.append_child(&entry);
        entry.scroll_into_view();
    }

    pub fn on_pointer_event(&mut self, raw_event: PointerEvent) {
        let cooked_events = self.gestures.process_event(&raw_event);
        for g in cooked_events {
            let (opt_ev, dirty) = self.scroller.handle_gesture(g);
            if let Some(e) = opt_ev {
                self.add_log(&format!("{:?}, {:?}", e, self.scroller.current_view.center));
            }
        }
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
    }
}

#[wasm_bindgen(start)]
pub fn init_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug);
    log::info!("Hello from wasm");
    Ok(())
}