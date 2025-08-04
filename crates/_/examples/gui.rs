use fontdue::Font;
use glutin::{
    event::{Event, MouseButton},
    window::Window,
};
use raui_core::{
    layout::CoordsMappingScaling,
    widget::{
        component::{
            image_box::ImageBoxProps, interactive::navigation::NavItemActive,
            text_box::TextBoxProps,
        },
        unit::{
            image::{ImageBoxColor, ImageBoxFrame, ImageBoxImageScaling, ImageBoxMaterial},
            text::{TextBoxFont, TextBoxHorizontalAlign, TextBoxVerticalAlign},
        },
        utils::Color,
    },
};
use raui_immediate_widgets::core::{
    containers::{content_box, nav_horizontal_box, vertical_box},
    image_box,
    interactive::button,
    text_box,
};
use spitfire_draw::{
    context::DrawContext,
    utils::{ShaderRef, Vertex},
};
use spitfire_glow::{
    app::{App, AppControl, AppState},
    graphics::{Graphics, Shader, Texture},
    renderer::{GlowBlending, GlowTextureFiltering, GlowTextureFormat},
};
use spitfire_gui::{context::GuiContext, interactions::GuiInteractionsInputs};
use spitfire_input::*;
use std::{fs::File, path::Path};

fn main() {
    App::<Vertex>::default().run(State::default());
}

#[derive(Default)]
struct State {
    draw: DrawContext,
    // We store GUI context that stores RAUI application with its engines,
    // as well as immediate mode context and rendering configuration.
    gui: GuiContext,
    input: InputContext,
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        graphics.state.color = [0.25, 0.25, 0.25, 1.0];
        graphics.state.main_camera.screen_alignment = 0.5.into();
        self.gui.coords_map_scaling = CoordsMappingScaling::FitToView(512.0.into(), false);
        self.gui.interactions.engine.deselect_when_no_button_found = true;
        self.gui.texture_filtering = GlowTextureFiltering::Linear;

        // Define input actions and axes that will be used by GUI.
        let pointer_x = InputAxisRef::default();
        let pointer_y = InputAxisRef::default();
        let pointer_trigger = InputActionRef::default();

        let inputs = GuiInteractionsInputs {
            pointer_position: ArrayInputCombinator::new([pointer_x.clone(), pointer_y.clone()]),
            pointer_trigger: pointer_trigger.clone(),
            ..Default::default()
        };
        self.gui.interactions.inputs = inputs;

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

        self.draw.shaders.insert(
            "text".into(),
            graphics
                .shader(Shader::TEXT_VERTEX, Shader::TEXT_FRAGMENT)
                .unwrap(),
        );

        self.draw.textures.insert(
            "ferris".into(),
            load_texture(graphics, "resources/ferris.png"),
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

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>, control: &mut AppControl) {
        self.draw.begin_frame(graphics);
        self.draw.push_shader(&ShaderRef::name("image"));
        self.draw.push_blending(GlowBlending::Alpha);

        self.gui.begin_frame();

        // We construct immediate-mode GUI tree using `raui-immediate-widgets`,
        // a library of RAUI immediate-mode widgets that focus on ergonomics of
        // defining GUI from code. You can call widget functions as long as it
        // happen between `GuiContext::begin_frame` and `GuiContext::end_frame`.
        // Note that you can achieve multi layer screens by having multiple root
        // widgets present - all of these are children of true root `content_box`
        // so you can also apply `ContentBoxItemLayout` props to them to layout
        // them however you like on the screen - it is useful mostly for modals
        // or floating windows, side panels, etc.
        vertical_box((), || {
            content_box((), || {
                image_box(ImageBoxProps {
                    material: ImageBoxMaterial::Color(ImageBoxColor {
                        color: Color {
                            r: 0.0,
                            g: 0.75,
                            b: 0.0,
                            a: 1.0,
                        },
                        scaling: ImageBoxImageScaling::Frame(ImageBoxFrame {
                            destination: 30.0.into(),
                            frame_only: true,
                            ..Default::default()
                        }),
                    }),
                    ..Default::default()
                });

                nav_horizontal_box((), || {
                    let response = button(NavItemActive, |_| {
                        text_box(TextBoxProps {
                            text: "Minimize".to_owned(),
                            horizontal_align: TextBoxHorizontalAlign::Center,
                            vertical_align: TextBoxVerticalAlign::Middle,
                            font: TextBoxFont {
                                name: "roboto".to_owned(),
                                size: 32.0,
                            },
                            color: Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.75,
                                a: 1.0,
                            },
                            ..Default::default()
                        });
                    });
                    if response.trigger_start() {
                        control.set_minimized(true);
                    }

                    let response = button(NavItemActive, |_| {
                        text_box(TextBoxProps {
                            text: if control.maximized() {
                                "Restore".to_owned()
                            } else {
                                "Maximize".to_owned()
                            },
                            horizontal_align: TextBoxHorizontalAlign::Center,
                            vertical_align: TextBoxVerticalAlign::Middle,
                            font: TextBoxFont {
                                name: "roboto".to_owned(),
                                size: 32.0,
                            },
                            color: Color {
                                r: 0.0,
                                g: 0.75,
                                b: 0.0,
                                a: 1.0,
                            },
                            ..Default::default()
                        });
                    });
                    if response.trigger_start() {
                        control.set_maximized(!control.maximized());
                    }

                    let response = button(NavItemActive, |_| {
                        text_box(TextBoxProps {
                            text: "Close".to_owned(),
                            horizontal_align: TextBoxHorizontalAlign::Center,
                            vertical_align: TextBoxVerticalAlign::Middle,
                            font: TextBoxFont {
                                name: "roboto".to_owned(),
                                size: 32.0,
                            },
                            color: Color {
                                r: 0.75,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            },
                            ..Default::default()
                        });
                    });
                    if response.trigger_start() {
                        control.close_requested = true;
                    }
                });
            });

            image_box(ImageBoxProps::image_aspect_ratio("ferris", false));
        });

        // Here we perform actual rendering of constructed GUI widgets.
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
