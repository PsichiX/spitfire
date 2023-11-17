use crate::prelude::{GlowVertexAttribs, Graphics};
use glow::{Context, HasContext};
use glutin::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Fullscreen, Window, WindowBuilder},
    ContextBuilder, ContextWrapper, PossiblyCurrent,
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
    pub refresh_on_event: bool,
    pub color: [f32; 3],
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
            refresh_on_event: false,
            color: [1.0, 1.0, 1.0],
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

    pub fn refresh_on_event(mut self, v: bool) -> Self {
        self.refresh_on_event = v;
        self
    }

    pub fn color(mut self, v: impl Into<[f32; 3]>) -> Self {
        self.color = v.into();
        self
    }
}

pub struct App<V: GlowVertexAttribs> {
    width: u32,
    height: u32,
    refresh_on_event: bool,
    event_loop: EventLoop<()>,
    context_wrapper: ContextWrapper<PossiblyCurrent, Window>,
    graphics: Graphics<V>,
}

impl<V: GlowVertexAttribs> Default for App<V> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<V: GlowVertexAttribs> App<V> {
    pub fn new(config: AppConfig) -> Self {
        let AppConfig {
            title,
            width,
            height,
            fullscreen,
            maximized,
            vsync,
            decorations,
            transparent,
            refresh_on_event,
            color,
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
        let context_wrapper = unsafe {
            ContextBuilder::new()
                .with_vsync(vsync)
                .with_double_buffer(Some(true))
                .with_hardware_acceleration(Some(true))
                .build_windowed(window_builder, &event_loop)
                .expect("Could not build windowed context wrapper!")
                .make_current()
                .expect("Could not make windowed context wrapper a current one!")
        };
        let context = unsafe {
            Context::from_loader_function(|name| context_wrapper.get_proc_address(name) as *const _)
        };
        let mut graphics = Graphics::<V>::new(context);
        graphics.color = color;
        Self {
            width,
            height,
            refresh_on_event,
            event_loop,
            context_wrapper,
            graphics,
        }
    }

    pub fn run<S: AppState<V>>(self, mut state: S) -> S {
        let App {
            mut width,
            mut height,
            refresh_on_event,
            mut event_loop,
            context_wrapper,
            mut graphics,
        } = self;
        let (context, mut window) = unsafe { context_wrapper.split() };
        state.on_init(&mut graphics);
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
                        graphics.main_camera.viewport_size.x = width as _;
                        graphics.main_camera.viewport_size.y = height as _;
                        graphics.prepare_frame();
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
        state
    }
}
