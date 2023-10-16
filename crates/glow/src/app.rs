use crate::prelude::{GlowVertexAttribs, Graphics};
use glow::Context;
use glutin::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Fullscreen, WindowBuilder},
    ContextBuilder,
};

pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
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
            vsync: false,
            refresh_on_event: false,
            color: [1.0, 1.0, 1.0],
        }
    }
}

impl AppConfig {
    pub fn build<S, V: GlowVertexAttribs>(self) -> App<S, V> {
        App::new(self)
    }
}

pub struct App<S, V: GlowVertexAttribs> {
    config: AppConfig,
    #[allow(clippy::type_complexity)]
    on_redraw: Option<Box<dyn FnMut(&mut Graphics<V>, &mut S)>>,
    #[allow(clippy::type_complexity)]
    on_event: Option<Box<dyn FnMut(Event<()>, &mut S, &mut bool)>>,
}

impl<S, V: GlowVertexAttribs> App<S, V> {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            on_redraw: None,
            on_event: None,
        }
    }

    pub fn on_redraw(mut self, f: impl FnMut(&mut Graphics<V>, &mut S) + 'static) -> Self {
        self.on_redraw = Some(Box::new(f));
        self
    }

    pub fn on_event(mut self, f: impl FnMut(Event<()>, &mut S, &mut bool) + 'static) -> Self {
        self.on_event = Some(Box::new(f));
        self
    }

    pub fn run(self, mut state: S) -> S {
        let App {
            config,
            mut on_redraw,
            mut on_event,
        } = self;
        let AppConfig {
            title,
            width,
            height,
            fullscreen,
            vsync,
            refresh_on_event,
            color,
        } = config;
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
                        if let Some(on_redraw) = on_redraw.as_mut() {
                            on_redraw(&mut graphics, &mut state);
                        } else {
                            let _ = graphics.draw::<1>();
                        }
                        let _ = context_wrapper.swap_buffers();
                        *control_flow = ControlFlow::Exit;
                    }
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(physical_size) => {
                            context_wrapper.resize(*physical_size);
                        }
                        WindowEvent::CloseRequested => {
                            running = false;
                        }
                        _ => {}
                    },
                    _ => {}
                }
                if let Some(on_event) = on_event.as_mut() {
                    on_event(event, &mut state, &mut running);
                }
            });
        }
        state
    }
}
