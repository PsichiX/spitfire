use fontdue::Font;
use glutin::{
    event::{Event, MouseButton},
    window::Window,
};
use rand::random;
use raui_core::widget::{
    component::{
        containers::wrap_box::WrapBoxProps, image_box::ImageBoxProps,
        interactive::navigation::NavItemActive, text_box::TextBoxProps,
    },
    unit::{
        flex::FlexBoxItemLayout,
        image::{ImageBoxAspectRatio, ImageBoxImage, ImageBoxMaterial},
        text::{TextBoxFont, TextBoxHorizontalAlign, TextBoxVerticalAlign},
    },
    utils::Color,
};
use raui_immediate_widgets::core::{
    containers::{horizontal_box, nav_vertical_box, vertical_box, wrap_box},
    image_box,
    interactive::button,
    text_box,
};
use spitfire_draw::prelude::*;
use spitfire_glow::prelude::*;
use spitfire_gui::prelude::*;
use spitfire_input::*;
use std::{borrow::Cow, cmp::Ordering, fs::File, path::Path};

fn main() {
    App::<Vertex>::default().run(State::default());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Element {
    Fire,
    Water,
    Grass,
}

impl Element {
    fn iter() -> impl Iterator<Item = Self> {
        [Self::Fire, Self::Water, Self::Grass].into_iter()
    }
}

// Used for converting randomly generated index to element
// for enemy move selection.
impl From<usize> for Element {
    fn from(value: usize) -> Self {
        match value % 3 {
            0 => Self::Fire,
            1 => Self::Water,
            2 => Self::Grass,
            _ => unreachable!(),
        }
    }
}

// Used to map element to its image asset name.
impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fire => write!(f, "fire"),
            Self::Water => write!(f, "water"),
            Self::Grass => write!(f, "grass"),
        }
    }
}

// Used to get icon color for given element.
impl From<Element> for Color {
    fn from(val: Element) -> Self {
        match val {
            Element::Fire => Color {
                r: 1.0,
                g: 0.5,
                b: 0.5,
                a: 1.0,
            },
            Element::Water => Color {
                r: 0.5,
                g: 0.5,
                b: 1.0,
                a: 1.0,
            },
            Element::Grass => Color {
                r: 0.5,
                g: 1.0,
                b: 0.5,
                a: 1.0,
            },
        }
    }
}

// Used to tell score of one element relative to another.
// Think of it as: self - player, other - enemy.
impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Fire, Self::Fire) | (Self::Water, Self::Water) | (Self::Grass, Self::Grass) => {
                Ordering::Equal
            }
            (Self::Fire, Self::Water) | (Self::Water, Self::Grass) | (Self::Grass, Self::Fire) => {
                Ordering::Less
            }
            (Self::Fire, Self::Grass) | (Self::Water, Self::Fire) | (Self::Grass, Self::Water) => {
                Ordering::Greater
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
enum GameState {
    #[default]
    SelectMove,
    TurnResult {
        player: Element,
        enemy: Element,
    },
}

#[derive(Default)]
struct State {
    draw: DrawContext,
    gui: GuiContext,
    input: InputContext,
    game_state: GameState,
    player_score: usize,
    enemy_score: usize,
}

impl State {
    fn load_shader(
        &mut self,
        graphics: &Graphics<Vertex>,
        name: impl Into<Cow<'static, str>>,
        vertex: &str,
        fragment: &str,
    ) {
        self.draw
            .shaders
            .insert(name.into(), graphics.shader(vertex, fragment).unwrap());
    }

    fn load_texture(
        &mut self,
        graphics: &Graphics<Vertex>,
        name: impl Into<Cow<'static, str>>,
        path: impl AsRef<Path>,
    ) {
        let file = File::open(path).unwrap();
        let decoder = png::Decoder::new(file);
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let bytes = &buf[..info.buffer_size()];
        self.draw.textures.insert(
            name.into(),
            graphics
                .texture(
                    info.width,
                    info.height,
                    1,
                    GlowTextureFormat::Rgba,
                    Some(bytes),
                )
                .unwrap(),
        );
    }

    fn load_font(&mut self, name: impl Into<Cow<'static, str>>, path: impl AsRef<Path>) {
        let bytes = std::fs::read(path).unwrap();
        self.draw
            .fonts
            .insert(name, Font::from_bytes(bytes, Default::default()).unwrap());
    }

    fn perform_turn(&mut self, element: Element) {
        let enemy = random::<usize>().into();
        self.game_state = GameState::TurnResult {
            player: element,
            enemy,
        };
        match element.cmp(&enemy) {
            Ordering::Less => {
                self.enemy_score += 1;
            }
            Ordering::Greater => {
                self.player_score += 1;
            }
            _ => {}
        }
    }

    fn draw_gui(&mut self) {
        let props = WrapBoxProps {
            margin: 32.0.into(),
            fill: true,
        };

        wrap_box(props, || {
            nav_vertical_box((), || {
                let title = match self.game_state {
                    GameState::SelectMove => "SELECT YOUR ELEMENT",
                    GameState::TurnResult { player, enemy } => match player.cmp(&enemy) {
                        Ordering::Less => "YOU LOST!",
                        Ordering::Equal => "IT'S A TIE!",
                        Ordering::Greater => "YOU WON!",
                    },
                };

                text_box((
                    FlexBoxItemLayout {
                        basis: Some(80.0),
                        grow: 0.0,
                        shrink: 0.0,
                        ..Default::default()
                    },
                    TextBoxProps {
                        text: title.to_owned(),
                        horizontal_align: TextBoxHorizontalAlign::Center,
                        font: TextBoxFont {
                            name: "roboto".to_owned(),
                            size: 64.0,
                        },
                        color: Color {
                            r: 0.8,
                            g: 0.8,
                            b: 0.8,
                            a: 1.0,
                        },
                        ..Default::default()
                    },
                ));

                self.game_state_screen();
                self.scores();
            });
        });
    }

    fn game_state_screen(&mut self) {
        match &self.game_state {
            GameState::SelectMove => {
                horizontal_box((), || {
                    for element in Element::iter() {
                        self.element_button(element);
                    }
                });
            }
            GameState::TurnResult { player, enemy } => {
                horizontal_box((), || {
                    self.element_result("Player:", *player);
                    self.element_result("Enemy:", *enemy);
                });

                let result = button(NavItemActive, |state| {
                    text_box(TextBoxProps {
                        text: "TRY AGAIN!".to_owned(),
                        horizontal_align: TextBoxHorizontalAlign::Center,
                        vertical_align: TextBoxVerticalAlign::Middle,
                        font: TextBoxFont {
                            name: "roboto".to_owned(),
                            size: 96.0,
                        },
                        color: Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: if state.state.selected { 0.75 } else { 1.0 },
                        },
                        ..Default::default()
                    });
                });

                if result.trigger_start() {
                    self.game_state = GameState::SelectMove;
                }
            }
        }
    }

    fn scores(&self) {
        let props = FlexBoxItemLayout {
            basis: Some(50.0),
            grow: 0.0,
            shrink: 0.0,
            ..Default::default()
        };

        horizontal_box(props, || {
            self.score("Player", self.player_score, TextBoxHorizontalAlign::Left);
            self.score("Enemy", self.enemy_score, TextBoxHorizontalAlign::Right);
        });
    }

    fn score(&self, title: &str, value: usize, horizontal_align: TextBoxHorizontalAlign) {
        text_box(TextBoxProps {
            text: format!("{}: {}", title, value),
            horizontal_align,
            vertical_align: TextBoxVerticalAlign::Bottom,
            font: TextBoxFont {
                name: "roboto".to_owned(),
                size: 48.0,
            },
            color: Color {
                r: 0.7,
                g: 0.7,
                b: 0.7,
                a: 1.0,
            },
            ..Default::default()
        });
    }

    fn element_icon(&self, element: Element) {
        image_box(ImageBoxProps {
            content_keep_aspect_ratio: Some(ImageBoxAspectRatio {
                horizontal_alignment: 0.5,
                vertical_alignment: 0.5,
                outside: false,
            }),
            material: ImageBoxMaterial::Image(ImageBoxImage {
                id: element.to_string(),
                tint: element.into(),
                ..Default::default()
            }),
            ..Default::default()
        });
    }

    fn element_button(&mut self, element: Element) {
        let response = button(NavItemActive, |state| {
            let mut tint: Color = element.into();
            if state.state.selected {
                tint.a = 0.75;
            }

            image_box(ImageBoxProps {
                content_keep_aspect_ratio: Some(ImageBoxAspectRatio {
                    horizontal_alignment: 0.5,
                    vertical_alignment: 0.5,
                    outside: false,
                }),
                material: ImageBoxMaterial::Image(ImageBoxImage {
                    id: element.to_string(),
                    tint,
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

        if response.trigger_start() {
            self.perform_turn(element);
        }
    }

    fn element_result(&self, title: impl ToString, element: Element) {
        vertical_box((), || {
            text_box((
                FlexBoxItemLayout {
                    basis: Some(70.0),
                    grow: 0.0,
                    shrink: 0.0,
                    ..Default::default()
                },
                TextBoxProps {
                    text: title.to_string(),
                    horizontal_align: TextBoxHorizontalAlign::Center,
                    font: TextBoxFont {
                        name: "roboto".to_owned(),
                        size: 64.0,
                    },
                    color: Color {
                        r: 0.8,
                        g: 0.8,
                        b: 0.8,
                        a: 1.0,
                    },
                    ..Default::default()
                },
            ));

            self.element_icon(element);
        });
    }
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>) {
        graphics.color = [0.25, 0.25, 0.25, 1.0];

        self.load_shader(
            graphics,
            "color",
            Shader::COLORED_VERTEX_2D,
            Shader::PASS_FRAGMENT,
        );
        self.load_shader(
            graphics,
            "image",
            Shader::TEXTURED_VERTEX_2D,
            Shader::TEXTURED_FRAGMENT,
        );
        self.load_shader(graphics, "text", Shader::TEXT_VERTEX, Shader::TEXT_FRAGMENT);

        self.load_texture(graphics, "fire", "resources/fire.png");
        self.load_texture(graphics, "water", "resources/water.png");
        self.load_texture(graphics, "grass", "resources/grass.png");

        self.load_font("roboto", "resources/Roboto-Regular.ttf");

        self.gui.interactions.engine.deselect_when_no_button_found = true;
        self.gui.texture_filtering = GlowTextureFiltering::Linear;

        // Define input actions and axes that will be used by GUI.
        let pointer_x = InputAxisRef::default();
        let pointer_y = InputAxisRef::default();
        let pointer_trigger = InputActionRef::default();

        // Setup GUI inputs set out of these inputs.
        let inputs = GuiInteractionsInputs {
            pointer_position: ArrayInputCombinator::new([pointer_x.clone(), pointer_y.clone()]),
            pointer_trigger: pointer_trigger.clone(),
            ..Default::default()
        };
        self.gui.interactions.inputs = inputs;

        // And setup input mappings that will update these inputs.
        self.input.push_mapping(
            InputMapping::default()
                .consume(InputConsume::Hit)
                .axis(VirtualAxis::MousePositionX, pointer_x)
                .axis(VirtualAxis::MousePositionY, pointer_y)
                .action(
                    VirtualAction::MouseButton(MouseButton::Left),
                    pointer_trigger,
                ),
        );
    }

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>) {
        self.draw.begin_frame(graphics);
        self.draw.push_shader(&ShaderRef::name("image"));
        self.draw.push_blending(GlowBlending::Alpha);

        self.gui.begin_frame();
        self.draw_gui();
        self.gui.end_frame(
            &mut self.draw,
            graphics,
            &ShaderRef::name("color"),
            &ShaderRef::name("image"),
            &ShaderRef::name("text"),
        );

        self.draw.end_frame();
        self.input.maintain();
    }

    fn on_event(&mut self, event: Event<()>, _: &mut Window) -> bool {
        if let Event::WindowEvent { event, .. } = event {
            self.input.on_event(&event);
        }
        true
    }
}

#[test]
fn test_elements() {
    assert_eq!(Element::Fire.cmp(&Element::Fire), Ordering::Equal);
    assert_eq!(Element::Fire.cmp(&Element::Water), Ordering::Less);
    assert_eq!(Element::Fire.cmp(&Element::Grass), Ordering::Greater);
    assert_eq!(Element::Water.cmp(&Element::Fire), Ordering::Greater);
    assert_eq!(Element::Water.cmp(&Element::Water), Ordering::Equal);
    assert_eq!(Element::Water.cmp(&Element::Grass), Ordering::Less);
    assert_eq!(Element::Grass.cmp(&Element::Fire), Ordering::Less);
    assert_eq!(Element::Grass.cmp(&Element::Water), Ordering::Greater);
    assert_eq!(Element::Grass.cmp(&Element::Grass), Ordering::Equal);
}
