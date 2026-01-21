use glam::UVec2;
use wasm_bindgen::prelude::*;
use crate::gputil::GPUContext;


pub mod gputil;
mod shaders;


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
    pub async fn init_from_canvas(canvas: web_sys::HtmlCanvasElement) -> Self {
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

#[wasm_bindgen(start)]
pub fn init_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug);
    log::info!("Hello from wasm");
    Ok(())
}