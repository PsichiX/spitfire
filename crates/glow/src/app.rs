use crate::{
    graphics::{Graphics, SCREEN_CAPTURE_TARGET},
    renderer::GlowVertexAttribs,
};
use glow::{Context, HasContext};
#[cfg(not(target_arch = "wasm32"))]
use glutin::{
    ContextBuilder, ContextWrapper, PossiblyCurrent,
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Fullscreen, Window, WindowBuilder},
};
#[cfg(target_arch = "wasm32")]
use std::{cell::Cell, rc::Rc};
#[cfg(target_arch = "wasm32")]
use web_sys::{
    AddEventListenerOptions, HtmlCanvasElement, PointerEvent, WebGl2RenderingContext,
    wasm_bindgen::{JsCast, closure::Closure},
};
#[cfg(target_arch = "wasm32")]
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::web::WindowExtWebSys,
    window::{Window, WindowBuilder},
};

#[allow(unused_variables)]
pub trait AppState<V: GlowVertexAttribs> {
    fn on_init(&mut self, graphics: &mut Graphics<V>, control: &mut AppControl) {}

    fn on_redraw(&mut self, graphics: &mut Graphics<V>, control: &mut AppControl) {}

    fn on_event(&mut self, event: Event<()>, window: &mut Window) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub maximized: bool,
    pub vsync: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub double_buffer: Option<bool>,
    pub hardware_acceleration: Option<bool>,
    pub refresh_on_event: bool,
    pub color: [f32; 4],
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "Spitfire Application".to_owned(),
            width: 1024,
            height: 576,
            fullscreen: false,
            maximized: false,
            vsync: false,
            decorations: true,
            transparent: false,
            double_buffer: Some(true),
            hardware_acceleration: Some(true),
            refresh_on_event: false,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl AppConfig {
    pub fn title(mut self, v: impl ToString) -> Self {
        self.title = v.to_string();
        self
    }

    pub fn width(mut self, v: u32) -> Self {
        self.width = v;
        self
    }

    pub fn height(mut self, v: u32) -> Self {
        self.height = v;
        self
    }

    pub fn fullscreen(mut self, v: bool) -> Self {
        self.fullscreen = v;
        self
    }

    pub fn maximized(mut self, v: bool) -> Self {
        self.maximized = v;
        self
    }

    pub fn vsync(mut self, v: bool) -> Self {
        self.vsync = v;
        self
    }

    pub fn decorations(mut self, v: bool) -> Self {
        self.decorations = v;
        self
    }

    pub fn transparent(mut self, v: bool) -> Self {
        self.transparent = v;
        self
    }

    pub fn double_buffer(mut self, v: Option<bool>) -> Self {
        self.double_buffer = v;
        self
    }

    pub fn hardware_acceleration(mut self, v: Option<bool>) -> Self {
        self.hardware_acceleration = v;
        self
    }

    pub fn refresh_on_event(mut self, v: bool) -> Self {
        self.refresh_on_event = v;
        self
    }

    pub fn color(mut self, v: impl Into<[f32; 4]>) -> Self {
        self.color = v.into();
        self
    }
}

pub struct App<V: GlowVertexAttribs> {
    refresh_on_event: bool,
    event_loop: EventLoop<()>,
    #[cfg(not(target_arch = "wasm32"))]
    context_wrapper: ContextWrapper<PossiblyCurrent, Window>,
    #[cfg(target_arch = "wasm32")]
    window: Window,
    graphics: Graphics<V>,
    control: AppControl,
}

impl<V: GlowVertexAttribs> Default for App<V> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<V: GlowVertexAttribs> App<V> {
    pub fn new(config: AppConfig) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let AppConfig {
            title,
            width,
            height,
            fullscreen,
            maximized,
            vsync,
            decorations,
            transparent,
            double_buffer,
            hardware_acceleration,
            refresh_on_event,
            color,
        } = config;
        #[cfg(target_arch = "wasm32")]
        let AppConfig {
            title,
            width,
            height,
            fullscreen,
            maximized,
            decorations,
            transparent,
            refresh_on_event,
            color,
            ..
        } = config;
        let event_loop = EventLoop::new();
        let window_builder = WindowBuilder::new()
            .with_title(title.as_str())
            .with_inner_size(LogicalSize::new(width, height))
            .with_maximized(maximized)
            .with_decorations(decorations)
            .with_transparent(transparent);
        #[cfg(not(target_arch = "wasm32"))]
        let (context_wrapper, context) = {
            let window_builder =
                window_builder.with_fullscreen(fullscreen.then_some(Fullscreen::Borderless(None)));
            let context_builder = ContextBuilder::new()
                .with_vsync(vsync)
                .with_double_buffer(double_buffer)
                .with_hardware_acceleration(hardware_acceleration);
            #[cfg(debug_assertions)]
            crate::console_log!("* GL {:#?}", context_builder);
            let context_wrapper = unsafe {
                context_builder
                    .build_windowed(window_builder, &event_loop)
                    .expect("Could not build windowed context wrapper!")
                    .make_current()
                    .expect("Could not make windowed context wrapper a current one!")
            };
            let context = unsafe {
                Context::from_loader_function(|name| {
                    context_wrapper.get_proc_address(name) as *const _
                })
            };
            (context_wrapper, context)
        };
        #[cfg(target_arch = "wasm32")]
        let (window, context) = {
            use winit::platform::web::WindowBuilderExtWebSys;
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("screen")
                .unwrap()
                .dyn_into::<HtmlCanvasElement>()
                .expect("DOM element is not HtmlCanvasElement");
            let window = window_builder
                .with_canvas(Some(canvas.clone()))
                .build(&event_loop)
                .expect("Could not build window!");
            let context = Context::from_webgl2_context(
                canvas
                    .get_context("webgl2")
                    .expect("Could not get WebGL 2 context!")
                    .expect("Could not get WebGL 2 context!")
                    .dyn_into::<WebGl2RenderingContext>()
                    .expect("DOM element is not WebGl2RenderingContext"),
            );
            (window, context)
        };
        let context_version = context.version();
        #[cfg(debug_assertions)]
        crate::console_log!("* GL Version: {:?}", context_version);
        if context_version.major < 3 {
            panic!("* Minimum GL version required is 3.0!");
        }
        let mut graphics = Graphics::<V>::new(context);
        graphics.state.color = color;
        Self {
            refresh_on_event,
            event_loop,
            #[cfg(not(target_arch = "wasm32"))]
            context_wrapper,
            #[cfg(target_arch = "wasm32")]
            window,
            graphics,
            control: AppControl {
                x: 0,
                y: 0,
                dirty_pos: false,
                width,
                height,
                dirty_size: false,
                minimized: false,
                dirty_minimized: false,
                maximized,
                dirty_maximized: false,
                fullscreen,
                dirty_fullscreen: false,
                close_requested: false,
            },
        }
    }

    pub fn run<S: AppState<V> + 'static>(self, mut state: S) {
        #[cfg(not(target_arch = "wasm32"))]
        let App {
            refresh_on_event,
            mut event_loop,
            context_wrapper,
            mut graphics,
            mut control,
        } = self;
        #[cfg(target_arch = "wasm32")]
        let App {
            refresh_on_event,
            event_loop,
            mut window,
            mut graphics,
            mut control,
        } = self;
        #[cfg(not(target_arch = "wasm32"))]
        let (context, mut window) = unsafe { context_wrapper.split() };
        if let Ok(pos) = window.outer_position() {
            control.x = pos.x;
            control.y = pos.y;
        }
        let size = window.inner_size();
        control.width = size.width;
        control.height = size.height;
        control.minimized = control.width == 0 || control.height == 0;
        control.maximized = window.is_maximized();
        state.on_init(&mut graphics, &mut control);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut running = true;
            while running {
                if control.close_requested {
                    break;
                }
                event_loop.run_return(|event, _, control_flow| {
                    if control.dirty_pos {
                        control.dirty_pos = false;
                        window.set_outer_position(LogicalPosition::new(control.x, control.y));
                    }
                    if control.dirty_size {
                        control.dirty_size = false;
                        window.set_inner_size(LogicalSize::new(control.width, control.height));
                    }
                    if control.dirty_minimized {
                        control.dirty_minimized = false;
                        window.set_minimized(control.minimized);
                    } else {
                        control.minimized = control.width == 0 || control.height == 0;
                    }
                    if control.dirty_maximized {
                        control.dirty_maximized = false;
                        window.set_maximized(control.maximized);
                    } else {
                        control.maximized = window.is_maximized();
                    }
                    if control.dirty_fullscreen {
                        control.dirty_fullscreen = false;
                        window.set_fullscreen(
                            control.fullscreen.then_some(Fullscreen::Borderless(None)),
                        );
                    } else {
                        control.fullscreen = window.fullscreen().is_some();
                    }
                    *control_flow = if refresh_on_event {
                        ControlFlow::Wait
                    } else {
                        ControlFlow::Poll
                    };
                    match &event {
                        Event::MainEventsCleared => {
                            unsafe {
                                graphics.context().unwrap().viewport(
                                    0,
                                    0,
                                    control.width as _,
                                    control.height as _,
                                );
                            }
                            graphics.state.main_camera.screen_size.x = control.width as _;
                            graphics.state.main_camera.screen_size.y = control.height as _;
                            let _ = graphics.prepare_frame(true);
                            state.on_redraw(&mut graphics, &mut control);
                            let _ = graphics.draw();
                            graphics.resolve_capture(SCREEN_CAPTURE_TARGET);
                            let _ = context.swap_buffers();
                            *control_flow = ControlFlow::Exit;
                        }
                        Event::WindowEvent { event, .. } => match event {
                            WindowEvent::Resized(physical_size) => {
                                context.resize(*physical_size);
                                control.width = physical_size.width;
                                control.height = physical_size.height;
                                control.minimized = control.width == 0 || control.height == 0;
                            }
                            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                context.resize(**new_inner_size);
                                control.width = new_inner_size.width;
                                control.height = new_inner_size.height;
                                control.minimized = control.width == 0 || control.height == 0;
                            }
                            WindowEvent::CloseRequested => {
                                running = false;
                                control.close_requested = true;
                            }
                            WindowEvent::Moved(physical_position) => {
                                control.x = physical_position.x;
                                control.y = physical_position.y;
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    if !state.on_event(event, &mut window) {
                        running = false;
                    }
                });
            }
            drop(graphics);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let pointer_position = track_pointer_position(window.canvas());
            let fullscreen_request = request_fullscreen_on_gesture(control.fullscreen);
            let fullscreen_changed = track_fullscreen_change();
            event_loop.run(move |mut event, _, control_flow| {
                if let Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } = &mut event
                    && let Some(pointer) = pointer_position.get()
                {
                    *position = pointer.to_physical(window.scale_factor());
                }
                *control_flow = if refresh_on_event {
                    ControlFlow::Wait
                } else {
                    ControlFlow::Poll
                };
                match &event {
                    Event::MainEventsCleared => {
                        let dom_window = web_sys::window().unwrap();
                        let width = dom_window.inner_width().unwrap().as_f64().unwrap().max(1.0);
                        let height = dom_window
                            .inner_height()
                            .unwrap()
                            .as_f64()
                            .unwrap()
                            .max(1.0);
                        control.x = 0;
                        control.y = 0;
                        control.width = width as _;
                        control.height = height as _;
                        control.maximized = true;
                        if control.dirty_fullscreen {
                            control.dirty_fullscreen = false;
                            fullscreen_request.set(Some(control.fullscreen));
                            apply_fullscreen_request(&fullscreen_request);
                        } else if fullscreen_changed.take() {
                            control.fullscreen = is_document_fullscreen();
                        }
                        let scaled_width = width * window.scale_factor();
                        let scaled_height = height * window.scale_factor();
                        window.set_inner_size(LogicalSize::new(width, height));
                        graphics.state.main_camera.screen_size.x = scaled_width as _;
                        graphics.state.main_camera.screen_size.y = scaled_height as _;
                        let _ = graphics.prepare_frame(true);
                        state.on_redraw(&mut graphics, &mut control);
                        let _ = graphics.draw();
                        graphics.resolve_capture(SCREEN_CAPTURE_TARGET);
                        window.request_redraw();
                    }
                    _ => {}
                }
                state.on_event(event, &mut window);
            });
        }
    }
}

/// Tracks the pointer position in canvas CSS pixels, from `clientX` and the canvas rect.
/// Winit 0.28 reads `offsetX` instead. Chromium divides `offsetX` of synthetic pointer events
/// by the device pixel ratio, so these events land left and up of the real position.
/// The listener captures on the DOM window, so it runs before the winit listener on the canvas.
#[cfg(target_arch = "wasm32")]
fn track_pointer_position(canvas: HtmlCanvasElement) -> Rc<Cell<Option<LogicalPosition<f64>>>> {
    let position = Rc::new(Cell::new(None));
    let tracked = position.clone();
    let listener = Closure::<dyn FnMut(PointerEvent)>::new(move |event: PointerEvent| {
        let rect = canvas.get_bounding_client_rect();
        tracked.set(Some(LogicalPosition::new(
            event.client_x() as f64 - rect.x(),
            event.client_y() as f64 - rect.y(),
        )));
    });
    let options = AddEventListenerOptions::new();
    options.set_capture(true);
    let dom_window = web_sys::window().unwrap();
    for name in ["pointermove", "pointerdown", "pointerup"] {
        dom_window
            .add_event_listener_with_callback_and_add_event_listener_options(
                name,
                listener.as_ref().unchecked_ref(),
                &options,
            )
            .unwrap();
    }
    listener.forget();
    position
}

/// Keeps a wanted fullscreen state until the next pointer or key release.
/// Browsers enter fullscreen only inside a user gesture, so a saved setting waits for the first one.
/// A runtime toggle comes from a click in the previous frame, which still gives transient activation.
#[cfg(target_arch = "wasm32")]
fn request_fullscreen_on_gesture(fullscreen: bool) -> Rc<Cell<Option<bool>>> {
    let request = Rc::new(Cell::new(fullscreen.then_some(true)));
    let pending = request.clone();
    let listener = Closure::<dyn FnMut()>::new(move || apply_fullscreen_request(&pending));
    let options = AddEventListenerOptions::new();
    options.set_capture(true);
    let dom_window = web_sys::window().unwrap();
    for name in ["pointerup", "keyup"] {
        dom_window
            .add_event_listener_with_callback_and_add_event_listener_options(
                name,
                listener.as_ref().unchecked_ref(),
                &options,
            )
            .unwrap();
    }
    listener.forget();
    request
}

#[cfg(target_arch = "wasm32")]
fn apply_fullscreen_request(request: &Cell<Option<bool>>) {
    let Some(wanted) = request.take() else {
        return;
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let active = document.fullscreen_element().is_some();
    if wanted && !active {
        if let Some(root) = document.document_element() {
            let _ = root.request_fullscreen();
        }
    } else if !wanted && active {
        document.exit_fullscreen();
    }
}

/// Flags each finished or failed fullscreen switch.
/// The switch is asynchronous, so the document state lags a few frames behind a request.
#[cfg(target_arch = "wasm32")]
fn track_fullscreen_change() -> Rc<Cell<bool>> {
    let changed = Rc::new(Cell::new(false));
    let flag = changed.clone();
    let listener = Closure::<dyn FnMut()>::new(move || flag.set(true));
    let document = web_sys::window().unwrap().document().unwrap();
    for name in ["fullscreenchange", "fullscreenerror"] {
        document
            .add_event_listener_with_callback(name, listener.as_ref().unchecked_ref())
            .unwrap();
    }
    listener.forget();
    changed
}

#[cfg(target_arch = "wasm32")]
fn is_document_fullscreen() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| document.fullscreen_element().is_some())
}

#[derive(Debug)]
pub struct AppControl {
    x: i32,
    y: i32,
    dirty_pos: bool,
    width: u32,
    height: u32,
    dirty_size: bool,
    minimized: bool,
    dirty_minimized: bool,
    maximized: bool,
    dirty_maximized: bool,
    fullscreen: bool,
    dirty_fullscreen: bool,
    pub close_requested: bool,
}

impl AppControl {
    pub fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        if self.x == x && self.y == y {
            return;
        }
        self.x = x;
        self.y = y;
        self.dirty_pos = true;
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        self.dirty_size = true;
    }

    pub fn minimized(&self) -> bool {
        self.minimized
    }

    pub fn set_minimized(&mut self, minimized: bool) {
        if self.minimized == minimized {
            return;
        }
        self.minimized = minimized;
        self.dirty_minimized = true;
    }

    pub fn maximized(&self) -> bool {
        self.maximized
    }

    pub fn set_maximized(&mut self, maximized: bool) {
        if self.maximized == maximized {
            return;
        }
        self.maximized = maximized;
        self.dirty_maximized = true;
    }

    pub fn fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if self.fullscreen == fullscreen {
            return;
        }
        self.fullscreen = fullscreen;
        self.dirty_fullscreen = true;
    }
}
