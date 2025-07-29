use glutin::{
    event::{Event, VirtualKeyCode},
    window::Window,
};
use spitfire_draw::{
    context::DrawContext,
    sprite::{Sprite, SpriteTexture},
    utils::{Drawable, ShaderRef, TextureRef, Vertex},
};
use spitfire_glow::{
    app::{App, AppControl, AppState},
    graphics::{Graphics, Shader, Texture},
    renderer::{GlowBlending, GlowTextureFormat},
};
use spitfire_input::*;
use std::{fs::File, path::Path, time::Instant};
use vek::{Quaternion, Rgba, Vec2};

fn main() {
    App::<Vertex>::default().run(State::new());
}

struct Player {
    input_move: CardinalInputCombinator,
    input_rotate: DualInputCombinator,
    input_camera_attached_to_ferris: InputActionRef,
    sprite: Sprite,
    speed: f32,
}

impl Player {
    fn new(texture: TextureRef, speed: f32, input: &mut InputContext) -> Self {
        // Spitfire's input primitives are actions and axes references,
        // which we ask for their state when needed.
        // They serve as logical input representation.
        let move_left = InputActionRef::default();
        let move_right = InputActionRef::default();
        let move_up = InputActionRef::default();
        let move_down = InputActionRef::default();
        let rotate_left = InputActionRef::default();
        let rotate_right = InputActionRef::default();
        let input_camera_attached_to_ferris = InputActionRef::default();

        // Once we create input references, we need to map them to physical
        // input actions and axes.
        // We can map same input references to multiple physical inputs.
        // Mapping itself does nothing, until we push it into input stack.
        input.push_mapping(
            InputMapping::default()
                .consume(InputConsume::Hit)
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::Left),
                    move_left.clone(),
                )
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::A),
                    move_left.clone(),
                )
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::Right),
                    move_right.clone(),
                )
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::D),
                    move_right.clone(),
                )
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::Up),
                    move_up.clone(),
                )
                .action(VirtualAction::KeyButton(VirtualKeyCode::W), move_up.clone())
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::Down),
                    move_down.clone(),
                )
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::S),
                    move_down.clone(),
                )
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::Q),
                    rotate_left.clone(),
                )
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::E),
                    rotate_right.clone(),
                )
                .action(
                    VirtualAction::KeyButton(VirtualKeyCode::Space),
                    input_camera_attached_to_ferris.clone(),
                ),
        );

        // Input combinators are used to ease mapping multiple reference
        // state into single output value.
        // Cardinal combinator is useful for things like player movement.
        let input_move = CardinalInputCombinator::new(move_left, move_right, move_up, move_down);
        let input_rotate = DualInputCombinator::new(rotate_left, rotate_right);

        let sprite = Sprite::single(SpriteTexture::new("u_image".into(), texture))
            .shader(ShaderRef::name("image"))
            .pivot(0.5.into())
            .blending(GlowBlending::Alpha);

        Self {
            input_move,
            input_rotate,
            input_camera_attached_to_ferris,
            sprite,
            speed,
        }
    }

    fn update(&mut self) {
        if !self.input_camera_attached_to_ferris.get().is_hold() {
            // By getting combinator values we get each of combined inputs state
            // at once and let combinator process them into single useful value.
            let input_move = Vec2::from(self.input_move.get())
                .try_normalized()
                .unwrap_or_default();
            let input_rotate = self.input_rotate.get();

            self.sprite.transform.position += input_move * self.speed;
            self.sprite.transform.orientation = self.sprite.transform.orientation
                * Quaternion::rotation_z(input_rotate * 5.0_f32.to_radians());
        }
    }

    fn draw(&self, draw: &mut DrawContext, graphics: &mut Graphics<Vertex>, ticked: bool) {
        if ticked && self.input_camera_attached_to_ferris.get().is_hold() {
            let input_move = Vec2::from(self.input_move.get())
                .try_normalized()
                .unwrap_or_default();
            let input_rotate = self.input_rotate.get();

            graphics.main_camera.transform.position += input_move * self.speed;
            graphics.main_camera.transform.orientation = graphics.main_camera.transform.orientation
                * Quaternion::rotation_z(input_rotate * 5.0_f32.to_radians());
        }

        self.sprite.draw(draw, graphics);
    }
}

struct State {
    draw: DrawContext,
    input: InputContext,
    tick: Instant,
    player: Player,
    input_exit: InputActionRef,
}

impl State {
    fn new() -> Self {
        // Since we can stack multiple input mappings, this means different
        // parts of the application can have their own input mappings being
        // present and reacting to input. And because of stack, received
        // input falls down that stack, and that input can be consumed either
        // entirely blocking next mappings from receiving it, or blocking
        // only those inputs that gets hit at some mappings level.
        let mut input = InputContext::default();
        let input_exit = InputActionRef::default();
        input.push_mapping(InputMapping::default().action(
            VirtualAction::KeyButton(VirtualKeyCode::Escape),
            input_exit.clone(),
        ));

        let player = Player::new(TextureRef::name("ferris"), 10.0, &mut input);

        Self {
            draw: Default::default(),
            input,
            tick: Instant::now(),
            player,
            input_exit,
        }
    }

    fn update(&mut self) {
        self.player.update();
    }

    fn draw(&mut self, graphics: &mut Graphics<Vertex>, ticked: bool) {
        Sprite::default()
            .shader(ShaderRef::name("color"))
            .size(1000.0.into())
            .pivot(0.5.into())
            .tint(Rgba::blue())
            .draw(&mut self.draw, graphics);

        self.player.draw(&mut self.draw, graphics, ticked);
    }
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        graphics.color = [0.25, 0.25, 0.25, 1.0];
        graphics.main_camera.screen_alignment = 0.5.into();

        self.draw.shaders.insert(
            "color".into(),
            graphics
                .shader(Shader::COLORED_VERTEX_2D, Shader::PASS_FRAGMENT)
                .unwrap(),
        );

        self.draw.shaders.insert(
            "image".into(),
            graphics
                .shader(Shader::TEXTURED_VERTEX_2D, Shader::TEXTURED_FRAGMENT)
                .unwrap(),
        );

        self.draw.textures.insert(
            "ferris".into(),
            load_texture(graphics, "resources/ferris.png"),
        );
    }

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        // We loosely simulate fixed update tick rate.
        let ticked = self.tick.elapsed().as_millis() > 16;
        if ticked {
            self.tick = Instant::now();
            self.update();
        }

        self.draw.begin_frame(graphics);
        self.draw.push_shader(&ShaderRef::name("image"));
        self.draw.push_blending(GlowBlending::Alpha);

        self.draw(graphics, ticked);

        self.draw.end_frame();
        // After frame ends, we need to maintain inputs stack to make its
        // mappings properly change states from pressed/released into
        // idle/hold, otherwise inputs would have only pressed/released
        // state, which would end up really bad for applciaiton logic.
        self.input.maintain();
    }

    fn on_event(&mut self, event: Event<()>, _: &mut Window) -> bool {
        if let Event::WindowEvent { event, .. } = event {
            // Here we apply received input changes for stack to update.
            self.input.on_event(&event);
        }

        // Here we read our application exit input and exit if pressed.
        !self.input_exit.get().is_pressed()
    }
}

fn load_texture(graphics: &Graphics<Vertex>, path: impl AsRef<Path>) -> Texture {
    let file = File::open(path).unwrap();
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..info.buffer_size()];
    graphics
        .texture(
            info.width,
            info.height,
            1,
            GlowTextureFormat::Rgba,
            Some(bytes),
        )
        .unwrap()
}
