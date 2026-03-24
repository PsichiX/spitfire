use fontdue::Font;
use glutin::{event::Event, window::Window};
use spitfire_draw::{
    context::DrawContext,
    text::Text,
    utils::{Drawable, ShaderRef, Vertex},
};
use spitfire_glow::{
    app::{App, AppControl, AppState},
    graphics::{Graphics, Shader},
    renderer::GlowBlending,
};
use spitfire_input::*;

fn main() {
    App::<Vertex>::default().run(State::new());
}

struct State {
    draw: DrawContext,
    input: InputContext,
    detector: InputActionDetector,
    action: Option<VirtualAction>,
}

impl State {
    fn new() -> Self {
        Self {
            draw: Default::default(),
            input: InputContext::default().with_gamepads(),
            detector: Default::default(),
            action: None,
        }
    }

    fn draw(&mut self, graphics: &mut Graphics<Vertex>) {
        let text = if let Some(action) = self.action {
            format!("Detected input: {action:?}")
        } else {
            "Press any key or button...".to_string()
        };
        let text = Text::new(ShaderRef::name("text"))
            .font("roboto")
            .size(30.0)
            .text(text)
            .tint([0.0, 0.8, 1.0, 1.0].into())
            .position([-450.0, 0.0].into());
        text.draw(&mut self.draw, graphics);
    }
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        graphics.state.color = [0.25, 0.25, 0.25, 1.0];
        graphics.state.main_camera.screen_alignment = 0.5.into();

        self.draw.shaders.insert(
            "text".into(),
            graphics
                .shader(Shader::TEXT_VERTEX, Shader::TEXT_FRAGMENT)
                .unwrap(),
        );

        self.draw.fonts.insert(
            "roboto",
            Font::from_bytes(
                include_bytes!("../../../resources/Roboto-Regular.ttf") as &[_],
                Default::default(),
            )
            .unwrap(),
        );
    }

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        self.draw.begin_frame(graphics);
        self.draw.push_shader(&ShaderRef::name("text"));
        self.draw.push_blending(GlowBlending::Alpha);

        self.draw(graphics);
        self.detector.gamepad_detect(&mut self.input, None);
        if let Some(action) = self.detector.try_consume() {
            self.action = Some(action);
        }

        self.draw.end_frame();
    }

    fn on_event(&mut self, event: Event<()>, _: &mut Window) -> bool {
        if let Event::WindowEvent { event, .. } = event {
            self.detector.window_detect(&mut self.input, &event);
            if let Some(action) = self.detector.try_consume() {
                self.action = Some(action);
            }
        }
        true
    }
}
