
use std::mem::replace;

use clone_all::clone_all;
use leptos::{html::{Canvas, tr}, prelude::*, task::spawn_local};
use glam::{DVec2, UVec2, dvec2, uvec2};
use web_sys::{PointerEvent, js_sys};

use crate::{gputil::GPUContext, grid::{GridParams, GridUniforms, GriddedRenderer}, pointer::GestureRecognizer, util::resize::{ResizeObserverHandle, auto_resize_canvas}, viewport::{ViewportScroller, WorldMouseEvent}};


struct GriddedDisplayState {
    pub gestures: GestureRecognizer,
    pub scroller: ViewportScroller,
    pub gpu: GPUContext,
    pub renderer: GriddedRenderer,
    pub dirty: bool,
}

#[component]
pub fn GriddedDisplay(
    grid_params: Signal<GridParams>,
    on_mouse: impl Fn(WorldMouseEvent) + Copy + 'static,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();

    let state_ref = StoredValue::<Option<GriddedDisplayState>, _>::new_local(None);

    let resize_handle = StoredValue::<Option<ResizeObserverHandle>, _>::new_local(None);

    let on_frame = move || {
        state_ref.try_update_value(|maybe_state|{
            if let Some(state) = maybe_state {
                state.dirty = false;

                let grid_uniforms = &GridUniforms {
                    viewport: state.scroller.current_view.to_uniforms(),
                    params: grid_params.get_untracked(),
                };
                let res = state.renderer.render(&state.gpu, grid_uniforms, move |_,_| {});
                if let Err(e) = res {
                    log::warn!("Error with swapchain: {}", e);
                }
            }
        });
    };

    let refresh = move || {
        let was_dirty = state_ref.try_update_value(|maybe_state| {
            if let Some(state) = maybe_state {
                replace(&mut state.dirty, true)
            } else {
                true
            }
        });
        if let Some(false) = was_dirty {
            request_animation_frame(on_frame);
        }
    };

    let on_pointer_event = move |raw_event: PointerEvent| {
        let need_refresh = state_ref.try_update_value(|maybe_state|{
            if let Some(state) = maybe_state {
                let cooked_events = state.gestures.process_event(&raw_event);
                let mut anydirty = false;
                for g in cooked_events {
                    let (opt_ev, dirty) = state.scroller.handle_gesture(g);
                    anydirty = anydirty || dirty;
                    if let Some(e) = opt_ev {
                        on_mouse(e);
                    }
                }
                anydirty
            } else {
                false
            }
        });
        if let Some(true) = need_refresh {
            refresh();
        }
    };

    let on_resize = move |css_size: DVec2, device_size: UVec2| {
        state_ref.try_update_value(|maybe_state|{
            if let Some(state) = maybe_state {
                let gestures = state.gestures.resize(css_size);
                for g in gestures {
                    let (opt_ev, dirty) = state.scroller.handle_gesture(g);
                    if let Some(e) = opt_ev {
                        on_mouse(e);
                    }
                }
                state.scroller.handle_resize(device_size);
                state.renderer.resize(&state.gpu, device_size);
            }
        });
        refresh();
    };

    canvas_ref.on_load(move |canvas| {
        spawn_local({clone_all!(canvas); async move {
            let wgpu_inst = wgpu::Instance::default();
            let surface = wgpu_inst.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).unwrap();
            let gpu = GPUContext::with_limits(
                wgpu_inst,
                Some(&surface),
                wgpu::Features::default(),
                Default::default(),
            ).await;
            log::info!("initialized gpu");

            let css_width = canvas.client_width();
            let css_height =  canvas.client_height();
            let gestures = GestureRecognizer::new(dvec2(css_width as f64, css_height as f64));
            let init_css_size = uvec2(css_width as u32, css_height as u32);
            let init_rad = dvec2(10.0, 10.0);
            let scroller = ViewportScroller::new(init_css_size, init_rad);

            let canvas_size = uvec2(canvas.width(), canvas.height());
            let renderer = GriddedRenderer::new(&gpu, surface, canvas_size);
            log::info!("initialized renderer");

            let state = GriddedDisplayState {
                gestures, scroller,
                gpu,
                renderer,
                dirty: true,
            };
            if let None = state_ref.try_set_value(Some(state)) {
                request_animation_frame(on_frame);
            }
        }});

        let resizer = auto_resize_canvas(&canvas, on_resize);
        resize_handle.try_set_value(Some(resizer));
    });
    
    view! {<canvas
        id="gridded_display"
        node_ref=canvas_ref
        on:pointermove=on_pointer_event
        on:pointerup=on_pointer_event
        on:pointerdown=on_pointer_event
        on:pointerenter=on_pointer_event
        on:pointerleave=on_pointer_event
        on:pointercancel=on_pointer_event
    />}
}