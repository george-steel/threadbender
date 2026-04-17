
use std::{fmt::Pointer, mem::replace, ops::{Deref, DerefMut}, sync::Arc};

use clone_all::clone_all;
use clothoid::spline::{ClothoidSplineCage, solve_clothoid_section, stage_clothoid_params};
use leptos::{html::{Canvas, tr}, prelude::*, tachys::view, task::spawn_local};
use glam::{DVec2, UVec2, dvec2, uvec2};
use web_sys::{MouseEvent, PointerEvent, WheelEvent, js_sys};

use crate::{gputil::GPUContext, line::DisplayHandle, pointer::GestureRecognizer, renderer::{GridParams, LineEditRenderer}, util::{Mailbox, resize::{ResizeObserverHandle, auto_resize_canvas}}, viewport::{ViewportScroller, ViewportWindow, WorldMouseEvent}};

#[derive(Clone)]
pub struct  SplineEditConnection {
    pub handles: ArcSignal<Vec<DisplayHandle>>,
    pub line: ArcSignal<ClothoidSplineCage>,
    pub on_mouse: Arc<dyn Fn(WorldMouseEvent, &ViewportWindow) + Send + Sync>,
}

#[component]
pub fn GriddedDisplay(
    grid_params: Signal<GridParams>,
    editing: Signal<Option<SplineEditConnection>>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let resize_handle = StoredValue::<Option<ResizeObserverHandle>, _>::new_local(None);

    let (viewport, set_viewport) = signal(ViewportWindow::PLACEHOLDER);
    let gesture_state = StoredValue::new(GestureRecognizer::new(dvec2(100.0, 100.0)));
    let scroller_state = StoredValue::new(ViewportScroller::placeholder());

    let renderer_state: StoredValue::<(Option<LineEditRenderer>, bool), _> = StoredValue::new_local((None, true));
    
    
    let need_redraw = ArcTrigger::new();
    let viewport_box = Mailbox::new_scoped(viewport.into(), need_redraw.clone());
    let grid_box = Mailbox::new_scoped(grid_params.into(), need_redraw.clone());
    let handles_box: Mailbox<Vec<DisplayHandle>> = Mailbox::new_scoped(ArcSignal::derive(move || {
        match editing.read().deref() {
            Some(conn) => conn.handles.get(),
            None => Vec::new(),
        }
    }), need_redraw.clone());
    let line_box: Mailbox<ClothoidSplineCage> = Mailbox::new_scoped(ArcSignal::derive(move || {
        match editing.read().deref() {
            Some(conn) => conn.line.get(),
            None => ClothoidSplineCage::new(),
        }
    }), need_redraw.clone());

    let on_frame = move || {
        if let Some((Some(renderer), pending)) = renderer_state.try_write_value().as_deref_mut() {
            *pending = false;

            if let Some(view) = viewport_box.get_new() {
                renderer.set_viewport(&view);
            }
            if let Some(grid) = grid_box.get_new() {
                renderer.set_grid_params(&grid);
            }
            if let Some(h) = handles_box.get_new() {
                renderer.set_handles(&h);
            }
            if let Some(line) = line_box.get_new() {
                let solution = line.solve();
                let segments = stage_clothoid_params(&line.points, &solution);
                renderer.set_splines(&segments);
            }

            let res = renderer.render();
            if let Err(e) = res {
                log::warn!("Error with swapchain: {}", e);
            }
        };
    };

    let refresh = move || {
        let was_pending = renderer_state.try_update_value(|(renderer, pending)| {
            if let Some(_) = renderer {
                replace(pending, true)
            } else {
                true
            }
        }).unwrap_or(true);
        if !was_pending {
            // if this was already dirty, a frame is already updoming
            request_animation_frame(on_frame);
        }
    };

    {clone_all!(need_redraw);
    Effect::new(move || {
        need_redraw.track();
        refresh();
    });}

    let on_pointer_event = move |raw_event: PointerEvent| {
        let cooked_events = gesture_state.write_value().process_event(&raw_event);
        for g in cooked_events {
            let (opt_ev, moved, view) = scroller_state.write_value().handle_gesture(g);
            if moved {
                set_viewport.set(view);
            }
            if let Some(e) = opt_ev {
                if let Some(conn) = editing.get_untracked() {
                    conn.on_mouse.deref()(e, &view);
                }
            }
        }
        raw_event.prevent_default();
    };

    let on_wheel_event = move |raw_event: WheelEvent| {
        if let Some((delta, clip)) = gesture_state.write_value().process_wheel(&raw_event) {
            let view = scroller_state.write_value().handle_wheel(delta, clip);
            set_viewport.set(view);
        }
    };

    let on_context_menu_event = move |raw_event: MouseEvent| {
        raw_event.prevent_default();
    };

    let on_resize = move |css_size: DVec2, device_size: UVec2| {
        let gestures = gesture_state.write_value().resize(css_size);
        for g in gestures {
            let (opt_ev, moved, view) = scroller_state.write_value().handle_gesture(g);
            if let Some(e) = opt_ev {
                if let Some(conn) = editing.get_untracked() {
                    conn.on_mouse.deref()(e, &view);
                }
            }
        }
        let view = scroller_state.write_value().handle_resize(device_size);
        set_viewport.set(view);

        if let (Some(renderer), _) = renderer_state.write_value().deref_mut() {
            renderer.resize(device_size);
        };
        need_redraw.notify();
    };


    canvas_ref.on_load(move |canvas| {
        let init_css_size = dvec2(canvas.client_width() as f64, canvas.client_height() as f64);
        let css_ratio = window().device_pixel_ratio();
        let init_dev_size = (init_css_size * css_ratio).round().as_uvec2();
        gesture_state.set_value(GestureRecognizer::new(init_css_size));

        let init_rad = dvec2(10.0, 10.0);
        scroller_state.set_value(ViewportScroller::new_from_dims(init_dev_size, css_ratio, init_rad));
        set_viewport.set(scroller_state.read_value().current_view);

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

            let canvas_size = uvec2(canvas.width(), canvas.height());
            let renderer = LineEditRenderer::new(&gpu, surface, canvas_size, &viewport.get_untracked(), &grid_params.get_untracked());
            log::info!("initialized renderer");

            if let None = renderer_state.try_set_value((Some(renderer), true)) {
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
        on:wheel=on_wheel_event
        on:contextmenu=on_context_menu_event
    />}
}