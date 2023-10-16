use crate::prelude::{GlowVertexAttribs, Graphics};
use glow::{Context, HasContext};
use glutin::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Fullscreen, WindowBuilder},
    ContextBuilder,
};

#[allow(unused_variables)]
pub trait AppState<V: GlowVertexAttribs> {
    fn on_init(&mut self, graphics: &mut Graphics<V>) {}

    fn on_redraw(&mut self, graphics: &mut Graphics<V>) {}

    fn on_event(&mut self, event: Event<()>) -> bool {
        true
    }
}

pub struct App {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub refresh_on_event: bool,
    pub color: [f32; 3],
}

impl Default for App {
    fn default() -> Self {
        Self {
            title: "Spitfire Application".to_owned(),
            width: 1024,
            height: 576,
            fullscreen: false,
            vsync: false,
            refresh_on_event: false,
            color: [1.0, 1.0, 1.0],
        }
    }
}

impl App {
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

    pub fn vsync(mut self, v: bool) -> Self {
        self.vsync = v;
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

    pub fn run<S: AppState<V>, V: GlowVertexAttribs, const TN: usize>(self, mut state: S) -> S {
        let App {
            title,
            mut width,
            mut height,
            fullscreen,
            vsync,
            refresh_on_event,
            color,
        } = self;
        let fullscreen = if fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };
        let mut event_loop = EventLoop::new();
        let window_builder = WindowBuilder::new()
            .with_title(title.as_str())
            .with_inner_size(LogicalSize::new(width, height))
            .with_fullscreen(fullscreen);
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
                        let _ = graphics.draw::<TN>();
                        let _ = context_wrapper.swap_buffers();
                        *control_flow = ControlFlow::Exit;
                    }
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(physical_size) => {
                            context_wrapper.resize(*physical_size);
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
                if !state.on_event(event) {
                    running = false;
                }
            });
        }
        state
    }
}
