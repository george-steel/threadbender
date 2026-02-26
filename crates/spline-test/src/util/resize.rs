use glam::{DVec2, UVec2, dvec2};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, ResizeObserver, ResizeObserverOptions, js_sys::{self, Object}};
use clone_all::clone_all;

#[wasm_bindgen]
extern "C" {
    # [wasm_bindgen (extends = js_sys :: Object , js_name = ResizeObserverEntry , typescript_type = "ResizeObserverEntry")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type ResizeObserverEntry;

    # [wasm_bindgen (structural , method , getter , js_class = "ResizeObserverEntry" , js_name = target)]
    pub fn target(this: &ResizeObserverEntry) -> web_sys::Element;

    # [wasm_bindgen (structural , method , getter , js_class = "ResizeObserverEntry" , js_name = contentRect)]
    pub fn content_rect(this: &ResizeObserverEntry) -> web_sys::DomRectReadOnly;

    # [wasm_bindgen (structural , method , getter , js_class = "ResizeObserverEntry" , js_name = borderBoxSize)]
    pub fn border_box_size(this: &ResizeObserverEntry) -> Vec<web_sys::ResizeObserverSize>;

    # [wasm_bindgen (structural , method , getter , js_class = "ResizeObserverEntry" , js_name = contentBoxSize)]
    pub fn content_box_size(this: &ResizeObserverEntry) -> Vec<web_sys::ResizeObserverSize>;

    # [wasm_bindgen (structural , method , getter , js_class = "ResizeObserverEntry" , js_name = devicePixelContentBoxSize)]
    pub fn device_pixel_content_box_size(this: &ResizeObserverEntry) -> Vec<web_sys::ResizeObserverSize>;
}


// Workaround for lack of exception handling in ResizeObserver
fn has_device_pixel_support() -> bool {
    thread_local! {
        static DEVICE_PIXEL_SUPPORT: bool = {
            #[wasm_bindgen]
            extern "C" {
                type ResizeObserverEntryExt;

                #[wasm_bindgen(js_class = ResizeObserverEntry, static_method_of = ResizeObserverEntryExt, getter)]
                fn prototype() -> Object;
            }

            let prototype = ResizeObserverEntryExt::prototype();
            let descriptor = Object::get_own_property_descriptor(
                &prototype,
                &JsValue::from_str("devicePixelContentBoxSize"),
            );
            !descriptor.is_undefined()
        };
    }

    DEVICE_PIXEL_SUPPORT.with(|support| *support)
}

// store this in the arena to clean up
pub struct ResizeObserverHandle{
    pub observer: ResizeObserver,
}

impl Drop for ResizeObserverHandle {
    fn drop(&mut self) {
        self.observer.disconnect();
    }
}

pub fn auto_resize_canvas(
    canvas: &HtmlCanvasElement,
    mut callback: impl FnMut(DVec2, UVec2) + 'static,
) -> ResizeObserverHandle {
    let not_safari = has_device_pixel_support();
    let observer_func = if not_safari {
        Closure::new({clone_all!(canvas); move |entries: Vec<ResizeObserverEntry>| {
            for entry in entries {
                let css_box = &entry.content_box_size()[0];
                let css_size = dvec2(css_box.inline_size(), css_box.block_size());
                let px_box = &entry.device_pixel_content_box_size()[0];
                let px_size = dvec2(px_box.inline_size(), px_box.block_size()).round().as_uvec2();
                canvas.set_width(px_size.x);
                canvas.set_height(px_size.y);
                callback(css_size, px_size);
            }
        }}).into_js_value()
    } else {
        Closure::new({clone_all!(canvas); move |entries: Vec<ResizeObserverEntry>| {
            for entry in entries {
                let css_box = &entry.content_box_size()[0];
                let css_size = dvec2(css_box.inline_size(), css_box.block_size());
                let px_ratio = web_sys::window().unwrap().device_pixel_ratio();
                let px_size = (px_ratio * css_size).round().as_uvec2();
                canvas.set_width(px_size.x);
                canvas.set_height(px_size.y);
                callback(css_size, px_size);
            }
        }}).into_js_value()
    };

    let observer = web_sys::ResizeObserver::new(observer_func.unchecked_ref()).unwrap();

    let options = ResizeObserverOptions::new();
    if not_safari {
        options.set_box(web_sys::ResizeObserverBoxOptions::DevicePixelContentBox);
    } else {
        options.set_box(web_sys::ResizeObserverBoxOptions::ContentBox);
    }

    observer.observe_with_options(&canvas, &options);
    ResizeObserverHandle { observer }
}