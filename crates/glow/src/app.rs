use crate::{graphics::Graphics, renderer::GlowVertexAttribs};
use glow::{Context, HasContext};
#[cfg(not(target_arch = "wasm32"))]
use glutin::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Fullscreen, Window, WindowBuilder},
    ContextBuilder, ContextWrapper, PossiblyCurrent,
};
#[cfg(target_arch = "wasm32")]
use web_sys::{wasm_bindgen::JsCast, HtmlCanvasElement, WebGl2RenderingContext};
#[cfg(target_arch = "wasm32")]
use winit::{
    dpi::LogicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowBuilder},
};

#[allow(unused_variables)]
pub trait AppState<V: GlowVertexAttribs> {
    fn on_init(&mut self, graphics: &mut Graphics<V>) {}

    fn on_redraw(&mut self, graphics: &mut Graphics<V>) {}

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
    #[cfg(not(target_arch = "wasm32"))]
    width: u32,
    #[cfg(not(target_arch = "wasm32"))]
    height: u32,
    refresh_on_event: bool,
    event_loop: EventLoop<()>,
    #[cfg(not(target_arch = "wasm32"))]
    context_wrapper: ContextWrapper<PossiblyCurrent, Window>,
    #[cfg(target_arch = "wasm32")]
    window: Window,
    graphics: Graphics<V>,
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
        let fullscreen = if fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };
        let event_loop = EventLoop::new();
        let window_builder = WindowBuilder::new()
            .with_title(title.as_str())
            .with_inner_size(LogicalSize::new(width, height))
            .with_fullscreen(fullscreen)
            .with_maximized(maximized)
            .with_decorations(decorations)
            .with_transparent(transparent);
        #[cfg(not(target_arch = "wasm32"))]
        let (context_wrapper, context) = {
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
        graphics.color = color;
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            width,
            #[cfg(not(target_arch = "wasm32"))]
            height,
            refresh_on_event,
            event_loop,
            #[cfg(not(target_arch = "wasm32"))]
            context_wrapper,
            #[cfg(target_arch = "wasm32")]
            window,
            graphics,
        }
    }

    pub fn run<S: AppState<V> + 'static>(self, mut state: S) {
        #[cfg(not(target_arch = "wasm32"))]
        let App {
            mut width,
            mut height,
            refresh_on_event,
            mut event_loop,
            context_wrapper,
            mut graphics,
        } = self;
        #[cfg(target_arch = "wasm32")]
        let App {
            refresh_on_event,
            event_loop,
            mut window,
            mut graphics,
        } = self;
        #[cfg(not(target_arch = "wasm32"))]
        let (context, mut window) = unsafe { context_wrapper.split() };
        state.on_init(&mut graphics);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut running = true;
            while running {
                event_loop.run_return(|event, _, control_flow| {
                    *control_flow = if refresh_on_event {
                        ControlFlow::Wait
                    } else {
                        ControlFlow::Poll
                    };
                    match &event {
                        Event::MainEventsCleared => {
                            unsafe {
                                graphics
                                    .context()
                                    .unwrap()
                                    .viewport(0, 0, width as _, height as _);
                            }
                            graphics.main_camera.screen_size.x = width as _;
                            graphics.main_camera.screen_size.y = height as _;
                            let _ = graphics.prepare_frame(true);
                            state.on_redraw(&mut graphics);
                            let _ = graphics.draw();
                            let _ = context.swap_buffers();
                            *control_flow = ControlFlow::Exit;
                        }
                        Event::WindowEvent { event, .. } => match event {
                            WindowEvent::Resized(physical_size) => {
                                context.resize(*physical_size);
                                width = physical_size.width;
                                height = physical_size.height;
                            }
                            WindowEvent::CloseRequested => {
                                running = false;
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
            event_loop.run(move |event, _, control_flow| {
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
                        let scaled_width = width * window.scale_factor();
                        let scaled_height = height * window.scale_factor();
                        window.set_inner_size(LogicalSize::new(width, height));
                        graphics.main_camera.screen_size.x = scaled_width as _;
                        graphics.main_camera.screen_size.y = scaled_height as _;
                        graphics.prepare_frame(true);
                        state.on_redraw(&mut graphics);
                        let _ = graphics.draw();
                        window.request_redraw();
                    }
                    _ => {}
                }
                state.on_event(event, &mut window);
            });
        }
    }
}
