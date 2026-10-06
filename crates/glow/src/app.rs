use crate::{
    graphics::{Graphics, SCREEN_CAPTURE_TARGET},
    renderer::GlowVertexAttribs,
};
use glow::{Context, HasContext};
#[cfg(not(target_arch = "wasm32"))]
use glutin::{
    api::egl::{
        config::Config as EglConfig, context::PossiblyCurrentContext as EglPossiblyCurrentContext,
        device::Device as EglDevice, display::Display as EglDisplay,
        surface::Surface as EglSurface,
    },
    config::{Config as GlutinConfig, ConfigSurfaceTypes, ConfigTemplateBuilder, GlConfig},
    context::{
        ContextApi, ContextAttributes, ContextAttributesBuilder, NotCurrentContext,
        NotCurrentGlContext, PossiblyCurrentContext, PossiblyCurrentGlContext, Version,
    },
    display::{Display as GlutinDisplay, GetGlDisplay, GlDisplay},
    surface::{
        GlSurface, PbufferSurface, Surface as GlutinSurface, SurfaceAttributesBuilder,
        SwapInterval, WindowSurface,
    },
};
#[cfg(not(target_arch = "wasm32"))]
use glutin_winit::{DisplayBuilder, GlWindow};
#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::marker::PhantomData;
#[cfg(target_arch = "wasm32")]
use std::{cell::Cell, rc::Rc};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};
#[cfg(target_arch = "wasm32")]
use web_sys::{
    AddEventListenerOptions, HtmlCanvasElement, PointerEvent, WebGl2RenderingContext,
    wasm_bindgen::{JsCast, closure::Closure},
};
#[cfg(not(target_arch = "wasm32"))]
use winit::window::Fullscreen;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize},
    event::{DeviceEvent, DeviceId, Event, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

#[allow(unused_variables)]
pub trait AppState<V: GlowVertexAttribs> {
    fn on_init(&mut self, graphics: &mut Graphics<V>, control: &mut AppControl) {}

    fn on_redraw(&mut self, graphics: &mut Graphics<V>, control: &mut AppControl) {}

    /// Gets each window event loop event. A headless app has no window and sends no events.
    fn on_event(&mut self, event: Event<()>, window: Option<&Window>) -> bool {
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
    config: AppConfig,
    _vertex: PhantomData<fn() -> V>,
}

impl<V: GlowVertexAttribs> Default for App<V> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<V: GlowVertexAttribs> App<V> {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            _vertex: PhantomData,
        }
    }

    pub fn run<S: AppState<V> + 'static>(self, state: S) {
        let event_loop = EventLoop::new().expect("Could not create event loop!");
        let runner = AppRunner {
            control: AppControl::new(&self.config),
            config: self.config,
            state,
            frame: None,
            #[cfg(target_arch = "wasm32")]
            web: None,
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut runner = runner;
            event_loop
                .run_app(&mut runner)
                .expect("Could not run event loop!");
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(runner);
        }
    }
}

/// Renders one frame of the state into the current framebuffer.
fn render_frame<V: GlowVertexAttribs, S: AppState<V>>(
    graphics: &mut Graphics<V>,
    state: &mut S,
    control: &mut AppControl,
    width: u32,
    height: u32,
) {
    graphics.state.main_camera.screen_size.x = width as _;
    graphics.state.main_camera.screen_size.y = height as _;
    let _ = graphics.prepare_frame(true);
    state.on_redraw(graphics, control);
    let _ = graphics.draw();
    graphics.resolve_capture(SCREEN_CAPTURE_TARGET);
}

fn window_attributes(config: &AppConfig) -> WindowAttributes {
    let attributes = Window::default_attributes()
        .with_title(config.title.as_str())
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_maximized(config.maximized)
        .with_decorations(config.decorations)
        .with_transparent(config.transparent);
    #[cfg(not(target_arch = "wasm32"))]
    let attributes =
        attributes.with_fullscreen(config.fullscreen.then_some(Fullscreen::Borderless(None)));
    attributes
}

fn check_context_version(context: &Context) {
    let context_version = context.version();
    #[cfg(debug_assertions)]
    crate::console_log!("* GL Version: {:?}", context_version);
    if context_version.major < 3 {
        panic!("* Minimum GL version required is 3.0!");
    }
}

/// Holds the window and the GL objects of a running app.
/// The fields drop in order, so the graphics release GL objects while the context still lives.
struct AppFrame<V: GlowVertexAttribs> {
    graphics: Graphics<V>,
    #[cfg(not(target_arch = "wasm32"))]
    gl_surface: GlutinSurface<WindowSurface>,
    #[cfg(not(target_arch = "wasm32"))]
    gl_context: PossiblyCurrentContext,
    window: Window,
}

impl<V: GlowVertexAttribs> AppFrame<V> {
    #[cfg(not(target_arch = "wasm32"))]
    fn new(config: &AppConfig, event_loop: &ActiveEventLoop) -> Self {
        let mut template = ConfigTemplateBuilder::new()
            .with_transparency(config.transparent)
            .prefer_hardware_accelerated(config.hardware_acceleration);
        if let Some(double_buffer) = config.double_buffer {
            template = template.with_single_buffering(!double_buffer);
        }
        let (window, gl_config) = DisplayBuilder::new()
            .with_window_attributes(Some(window_attributes(config)))
            .build(event_loop, template, |configs| {
                configs
                    .min_by_key(|config| config.num_samples())
                    .expect("Could not find any GL config!")
            })
            .expect("Could not build window with GL config!");
        let window = window.expect("Could not create window!");
        let window_handle = window
            .window_handle()
            .expect("Could not get window handle!")
            .as_raw();
        let display = gl_config.display();
        let surface_attributes = window
            .build_surface_attributes(Default::default())
            .expect("Could not build window surface attributes!");
        let gl_surface = unsafe { display.create_window_surface(&gl_config, &surface_attributes) }
            .expect("Could not create window surface!");
        let gl_context = create_gl_context(&display, &gl_config, Some(window_handle))
            .make_current(&gl_surface)
            .expect("Could not make GL context current!");
        let swap_interval = if config.vsync {
            SwapInterval::Wait(NonZeroU32::MIN)
        } else {
            SwapInterval::DontWait
        };
        let _ = gl_surface.set_swap_interval(&gl_context, swap_interval);
        let context =
            unsafe { Context::from_loader_function_cstr(|name| display.get_proc_address(name)) };
        check_context_version(&context);
        let mut graphics = Graphics::<V>::new(context);
        graphics.state.color = config.color;
        Self {
            graphics,
            gl_surface,
            gl_context,
            window,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn new(config: &AppConfig, event_loop: &ActiveEventLoop) -> Self {
        use winit::platform::web::WindowAttributesExtWebSys;
        let canvas = screen_canvas();
        let window = event_loop
            .create_window(window_attributes(config).with_canvas(Some(canvas.clone())))
            .expect("Could not build window!");
        let context = Context::from_webgl2_context(
            canvas
                .get_context("webgl2")
                .expect("Could not get WebGL 2 context!")
                .expect("Could not get WebGL 2 context!")
                .dyn_into::<WebGl2RenderingContext>()
                .expect("DOM element is not WebGl2RenderingContext"),
        );
        check_context_version(&context);
        let mut graphics = Graphics::<V>::new(context);
        graphics.state.color = config.color;
        Self { graphics, window }
    }

    fn resize(&mut self, width: u32, height: u32) {
        #[cfg(not(target_arch = "wasm32"))]
        if let (Some(width), Some(height)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.gl_surface.resize(&self.gl_context, width, height);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (width, height);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn create_gl_context(
    display: &GlutinDisplay,
    config: &GlutinConfig,
    window_handle: Option<RawWindowHandle>,
) -> NotCurrentContext {
    context_attempts(window_handle)
        .iter()
        .find_map(|attributes| unsafe { display.create_context(config, attributes) }.ok())
        .expect("Could not create GL context!")
}

/// Lists GL context requests from the newest desktop GL version down to GLES.
/// A request with no version gives a 3.x context on some drivers, so each version is asked for first.
#[cfg(not(target_arch = "wasm32"))]
fn context_attempts(window_handle: Option<RawWindowHandle>) -> Vec<ContextAttributes> {
    [(4, 6), (4, 5), (4, 3), (4, 1), (3, 3)]
        .into_iter()
        .map(|(major, minor)| ContextApi::OpenGl(Some(Version::new(major, minor))))
        .chain([
            ContextApi::OpenGl(None),
            ContextApi::Gles(Some(Version::new(3, 0))),
            ContextApi::Gles(None),
        ])
        .map(|api| {
            ContextAttributesBuilder::new()
                .with_context_api(api)
                .build(window_handle)
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn screen_canvas() -> HtmlCanvasElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("screen")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .expect("DOM element is not HtmlCanvasElement")
}

#[cfg(target_arch = "wasm32")]
struct WebTracking {
    pointer_position: Rc<Cell<Option<LogicalPosition<f64>>>>,
    fullscreen_request: Rc<Cell<Option<bool>>>,
    fullscreen_changed: Rc<Cell<bool>>,
}

struct AppRunner<V: GlowVertexAttribs, S: AppState<V>> {
    config: AppConfig,
    state: S,
    control: AppControl,
    frame: Option<AppFrame<V>>,
    #[cfg(target_arch = "wasm32")]
    web: Option<WebTracking>,
}

impl<V: GlowVertexAttribs, S: AppState<V>> AppRunner<V, S> {
    fn forward_event(&mut self, event_loop: &ActiveEventLoop, event: Event<()>) {
        let window = self.frame.as_ref().map(|frame| &frame.window);
        if !self.state.on_event(event, window) {
            event_loop.exit();
        }
    }

    fn control_flow(&self) -> ControlFlow {
        if self.config.refresh_on_event {
            ControlFlow::Wait
        } else {
            ControlFlow::Poll
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sync_window(&mut self) {
        let Some(frame) = self.frame.as_mut() else {
            return;
        };
        let control = &mut self.control;
        if control.dirty_pos {
            control.dirty_pos = false;
            frame
                .window
                .set_outer_position(LogicalPosition::new(control.x, control.y));
        }
        if control.dirty_size {
            control.dirty_size = false;
            if let Some(size) = frame
                .window
                .request_inner_size(LogicalSize::new(control.width, control.height))
            {
                control.width = size.width;
                control.height = size.height;
                frame.resize(size.width, size.height);
            }
        }
        if control.dirty_minimized {
            control.dirty_minimized = false;
            frame.window.set_minimized(control.minimized);
        } else {
            control.minimized = control.width == 0 || control.height == 0;
        }
        if control.dirty_maximized {
            control.dirty_maximized = false;
            frame.window.set_maximized(control.maximized);
        } else {
            control.maximized = frame.window.is_maximized();
        }
        if control.dirty_fullscreen {
            control.dirty_fullscreen = false;
            frame
                .window
                .set_fullscreen(control.fullscreen.then_some(Fullscreen::Borderless(None)));
        } else {
            control.fullscreen = frame.window.fullscreen().is_some();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn redraw(&mut self) {
        let Some(frame) = self.frame.as_mut() else {
            return;
        };
        let (width, height) = (self.control.width, self.control.height);
        render_frame(
            &mut frame.graphics,
            &mut self.state,
            &mut self.control,
            width,
            height,
        );
        let _ = frame.gl_surface.swap_buffers(&frame.gl_context);
    }

    #[cfg(target_arch = "wasm32")]
    fn redraw(&mut self) {
        let (Some(frame), Some(web)) = (self.frame.as_mut(), self.web.as_ref()) else {
            return;
        };
        let dom_window = web_sys::window().unwrap();
        let width = dom_window.inner_width().unwrap().as_f64().unwrap().max(1.0);
        let height = dom_window
            .inner_height()
            .unwrap()
            .as_f64()
            .unwrap()
            .max(1.0);
        let control = &mut self.control;
        control.x = 0;
        control.y = 0;
        control.width = width as _;
        control.height = height as _;
        control.maximized = true;
        if control.dirty_fullscreen {
            control.dirty_fullscreen = false;
            web.fullscreen_request.set(Some(control.fullscreen));
            apply_fullscreen_request(&web.fullscreen_request);
        } else if web.fullscreen_changed.take() {
            control.fullscreen = is_document_fullscreen();
        }
        let scale_factor = frame.window.scale_factor();
        let _ = frame
            .window
            .request_inner_size(LogicalSize::new(width, height));
        render_frame(
            &mut frame.graphics,
            &mut self.state,
            &mut self.control,
            (width * scale_factor) as _,
            (height * scale_factor) as _,
        );
        frame.window.request_redraw();
    }
}

impl<V: GlowVertexAttribs, S: AppState<V>> ApplicationHandler for AppRunner<V, S> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.forward_event(event_loop, Event::NewEvents(cause));
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.frame.is_none() {
            let mut frame = AppFrame::<V>::new(&self.config, event_loop);
            if let Ok(pos) = frame.window.outer_position() {
                self.control.x = pos.x;
                self.control.y = pos.y;
            }
            let size = frame.window.inner_size();
            self.control.width = size.width;
            self.control.height = size.height;
            self.control.minimized = size.width == 0 || size.height == 0;
            self.control.maximized = frame.window.is_maximized();
            self.state.on_init(&mut frame.graphics, &mut self.control);
            #[cfg(target_arch = "wasm32")]
            {
                self.web = Some(WebTracking {
                    pointer_position: track_pointer_position(screen_canvas()),
                    fullscreen_request: request_fullscreen_on_gesture(self.control.fullscreen),
                    fullscreen_changed: track_fullscreen_change(),
                });
            }
            self.frame = Some(frame);
        }
        event_loop.set_control_flow(self.control_flow());
        self.forward_event(event_loop, Event::Resumed);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.forward_event(event_loop, Event::Suspended);
    }

    #[allow(unused_mut)]
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        mut event: WindowEvent,
    ) {
        match &mut event {
            WindowEvent::Resized(size) => {
                self.control.width = size.width;
                self.control.height = size.height;
                self.control.minimized = size.width == 0 || size.height == 0;
                if let Some(frame) = self.frame.as_mut() {
                    frame.resize(size.width, size.height);
                }
            }
            WindowEvent::CloseRequested => {
                self.control.close_requested = true;
                event_loop.exit();
            }
            WindowEvent::Moved(position) => {
                self.control.x = position.x;
                self.control.y = position.y;
            }
            #[cfg(target_arch = "wasm32")]
            WindowEvent::CursorMoved { position, .. } => {
                if let (Some(frame), Some(web)) = (self.frame.as_ref(), self.web.as_ref())
                    && let Some(pointer) = web.pointer_position.get()
                {
                    *position = pointer.to_physical(frame.window.scale_factor());
                }
            }
            _ => {}
        }
        self.forward_event(event_loop, Event::WindowEvent { window_id, event });
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.forward_event(event_loop, Event::DeviceEvent { device_id, event });
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(self.control_flow());
        #[cfg(not(target_arch = "wasm32"))]
        self.sync_window();
        self.redraw();
        if self.control.close_requested {
            event_loop.exit();
        }
        self.forward_event(event_loop, Event::AboutToWait);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.forward_event(event_loop, Event::LoopExiting);
        self.frame = None;
    }
}

/// Runs an app without a window, so it works on a machine with no display.
/// It renders into an EGL pbuffer, which acts as the default framebuffer, so screen captures work.
/// On a machine with no GPU, Mesa gives a software EGL device (llvmpipe).
/// `hardware_acceleration` set to `Some(false)` tries software devices first, `Some(true)` tries them last.
/// Set `SPITFIRE_EGL_DEVICE` to an index to use only that device.
/// With `vsync` on, the loop sleeps to keep at most 60 frames per second.
#[cfg(not(target_arch = "wasm32"))]
pub struct HeadlessApp<V: GlowVertexAttribs> {
    config: AppConfig,
    _vertex: PhantomData<fn() -> V>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<V: GlowVertexAttribs> Default for HeadlessApp<V> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<V: GlowVertexAttribs> HeadlessApp<V> {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            _vertex: PhantomData,
        }
    }

    pub fn run<S: AppState<V>>(self, mut state: S) {
        const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / 60);
        let mut control = AppControl::new(&self.config);
        control.maximized = false;
        control.fullscreen = false;
        let mut gl = HeadlessGl::new(
            control.width,
            control.height,
            self.config.hardware_acceleration,
        )
        .unwrap_or_else(|error| panic!("Could not create headless GL context: {error}"));
        let context =
            unsafe { Context::from_loader_function_cstr(|name| gl.display.get_proc_address(name)) };
        check_context_version(&context);
        let mut graphics = Graphics::<V>::new(context);
        graphics.state.color = self.config.color;
        state.on_init(&mut graphics, &mut control);
        while !control.close_requested {
            let frame_start = Instant::now();
            control.dirty_pos = false;
            control.dirty_minimized = false;
            control.dirty_maximized = false;
            control.dirty_fullscreen = false;
            if control.dirty_size {
                control.dirty_size = false;
                control.width = control.width.max(1);
                control.height = control.height.max(1);
                if let Err(error) = gl.resize(control.width, control.height) {
                    panic!("Could not resize headless GL surface: {error}");
                }
            }
            let (width, height) = (control.width, control.height);
            render_frame(&mut graphics, &mut state, &mut control, width, height);
            if let Some(context) = graphics.context() {
                unsafe { context.flush() };
            }
            if self.config.vsync {
                std::thread::sleep(FRAME_TIME.saturating_sub(frame_start.elapsed()));
            }
        }
        drop(graphics);
    }
}

/// Holds the EGL objects of a headless app.
/// The surface drops before the context, and the context drops before the display.
#[cfg(not(target_arch = "wasm32"))]
struct HeadlessGl {
    surface: EglSurface<PbufferSurface>,
    context: EglPossiblyCurrentContext,
    config: EglConfig,
    display: EglDisplay,
}

#[cfg(not(target_arch = "wasm32"))]
impl HeadlessGl {
    fn new(width: u32, height: u32, hardware_acceleration: Option<bool>) -> Result<Self, String> {
        let devices = EglDevice::query_devices()
            .map_err(|error| format!("Could not query EGL devices: {error}"))?
            .collect::<Vec<_>>();
        let selected = std::env::var("SPITFIRE_EGL_DEVICE")
            .ok()
            .and_then(|index| index.parse::<usize>().ok());
        let mut order = (0..devices.len()).collect::<Vec<_>>();
        if let Some(hardware_acceleration) = hardware_acceleration {
            order.sort_by_key(|index| {
                let software = devices[*index]
                    .extensions()
                    .contains("EGL_MESA_device_software");
                software == hardware_acceleration
            });
        }
        let mut errors = vec![];
        for index in order {
            let device = &devices[index];
            if selected.is_some_and(|selected| selected != index) {
                continue;
            }
            match Self::with_device(device, width, height) {
                Ok(result) => {
                    #[cfg(debug_assertions)]
                    crate::console_log!(
                        "* EGL device #{}: {} ({})",
                        index,
                        device.name().unwrap_or("unknown"),
                        device.vendor().unwrap_or("unknown vendor"),
                    );
                    return Ok(result);
                }
                Err(error) => errors.push(format!("device #{index}: {error}")),
            }
        }
        if errors.is_empty() {
            Err("No EGL device found".to_owned())
        } else {
            Err(errors.join(", "))
        }
    }

    fn with_device(device: &EglDevice, width: u32, height: u32) -> Result<Self, String> {
        let display = unsafe { EglDisplay::with_device(device, None) }
            .map_err(|error| format!("Could not create EGL display: {error}"))?;
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_surface_type(ConfigSurfaceTypes::PBUFFER)
            .build();
        let config = unsafe { display.find_configs(template) }
            .map_err(|error| format!("Could not find EGL configs: {error}"))?
            .min_by_key(|config| config.num_samples())
            .ok_or_else(|| "No EGL config supports pbuffers".to_owned())?;
        let context = context_attempts(None)
            .iter()
            .find_map(|attributes| unsafe { display.create_context(&config, attributes) }.ok())
            .ok_or_else(|| "Could not create EGL context".to_owned())?;
        let surface = Self::create_surface(&display, &config, width, height)?;
        let context = context
            .make_current(&surface)
            .map_err(|error| format!("Could not make EGL context current: {error}"))?;
        Ok(Self {
            surface,
            context,
            config,
            display,
        })
    }

    fn create_surface(
        display: &EglDisplay,
        config: &EglConfig,
        width: u32,
        height: u32,
    ) -> Result<EglSurface<PbufferSurface>, String> {
        let width = NonZeroU32::new(width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(height).unwrap_or(NonZeroU32::MIN);
        let attributes = SurfaceAttributesBuilder::<PbufferSurface>::new().build(width, height);
        unsafe { display.create_pbuffer_surface(config, &attributes) }
            .map_err(|error| format!("Could not create EGL pbuffer surface: {error}"))
    }

    /// Replaces the pbuffer, because EGL cannot resize a pbuffer surface.
    fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        let surface = Self::create_surface(&self.display, &self.config, width, height)?;
        self.context
            .make_current(&surface)
            .map_err(|error| format!("Could not make EGL context current: {error}"))?;
        self.surface = surface;
        Ok(())
    }
}

/// Tracks the pointer position in canvas CSS pixels, from `clientX` and the canvas rect.
/// Winit reads `offsetX` instead. Chromium divides `offsetX` of synthetic pointer events
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
    fn new(config: &AppConfig) -> Self {
        Self {
            x: 0,
            y: 0,
            dirty_pos: false,
            width: config.width,
            height: config.height,
            dirty_size: false,
            minimized: false,
            dirty_minimized: false,
            maximized: config.maximized,
            dirty_maximized: false,
            fullscreen: config.fullscreen,
            dirty_fullscreen: false,
            close_requested: false,
        }
    }

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
