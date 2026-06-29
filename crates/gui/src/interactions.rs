use raui_core::{
    application::Application,
    interactive::{
        InteractionsEngine,
        default_interactions_engine::{
            DefaultInteractionsEngine, DefaultInteractionsEngineResult, Interaction, PointerButton,
        },
    },
    layout::CoordsMapping,
    widget::{
        component::interactive::navigation::{NavJump, NavScroll, NavSignal, NavTextChange},
        utils::Vec2,
    },
};
use spitfire_input::{ArrayInputCombinator, InputActionRef, InputCharactersRef, InputCombinator};

const ZERO_THRESHOLD: f32 = 1.0e-6;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuiCardinalDirection {
    #[default]
    None,
    Up,
    Down,
    Left,
    Right,
}

impl GuiCardinalDirection {
    pub fn new(direction: [f32; 2], mut threshold: f32) -> Self {
        threshold = threshold.max(ZERO_THRESHOLD);
        let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
        if length < threshold {
            GuiCardinalDirection::None
        } else {
            let normalized = [direction[0] / length, direction[1] / length];
            let abs_x = normalized[0].abs();
            let abs_y = normalized[1].abs();
            if abs_x > abs_y {
                if normalized[0] > 0.0 {
                    GuiCardinalDirection::Right
                } else {
                    GuiCardinalDirection::Left
                }
            } else {
                if normalized[1] > 0.0 {
                    GuiCardinalDirection::Down
                } else {
                    GuiCardinalDirection::Up
                }
            }
        }
    }
}

pub struct GuiDefaultDirectionCombinator(pub InputCombinator<GuiCardinalDirection>);

impl GuiDefaultDirectionCombinator {
    pub fn new(
        left: InputActionRef,
        right: InputActionRef,
        up: InputActionRef,
        down: InputActionRef,
        threshold: f32,
    ) -> Self {
        Self(InputCombinator::new(move || {
            let direction = [
                if left.get().is_pressed() {
                    -1.0
                } else if right.get().is_pressed() {
                    1.0
                } else {
                    0.0
                },
                if up.get().is_pressed() {
                    1.0
                } else if down.get().is_pressed() {
                    -1.0
                } else {
                    0.0
                },
            ];
            GuiCardinalDirection::new(direction, threshold)
        }))
    }

    pub fn get(&self) -> GuiCardinalDirection {
        self.0.get()
    }
}

#[derive(Default)]
pub struct GuiInteractionsInputs {
    pub trigger: InputActionRef,
    pub context: InputActionRef,
    pub cancel: InputActionRef,
    pub direction: InputCombinator<GuiCardinalDirection>,
    pub prev: InputActionRef,
    pub next: InputActionRef,
    pub text: InputCharactersRef,
    pub text_start: InputActionRef,
    pub text_end: InputActionRef,
    pub text_delete_left: InputActionRef,
    pub text_delete_right: InputActionRef,
    pub pointer_position: ArrayInputCombinator<2>,
    pub pointer_trigger: InputActionRef,
    pub pointer_context: InputActionRef,
    pub scroll: ArrayInputCombinator<2>,
}

#[derive(Default)]
pub struct GuiInteractionsEngine {
    pub inputs: GuiInteractionsInputs,
    pub engine: DefaultInteractionsEngine,
    cached_pointer_position: Vec2,
}

impl GuiInteractionsEngine {
    pub fn maintain(&mut self, mapping: &CoordsMapping) {
        if self.engine.focused_text_input().is_some() {
            if let Some(mut text) = self.inputs.text.write() {
                for character in text.take().chars() {
                    self.engine
                        .interact(Interaction::Navigate(NavSignal::TextChange(
                            NavTextChange::InsertCharacter(character),
                        )));
                }
            }
            match self.inputs.direction.get() {
                GuiCardinalDirection::Left => {
                    self.engine
                        .interact(Interaction::Navigate(NavSignal::TextChange(
                            NavTextChange::MoveCursorLeft,
                        )));
                }
                GuiCardinalDirection::Right => {
                    self.engine
                        .interact(Interaction::Navigate(NavSignal::TextChange(
                            NavTextChange::MoveCursorRight,
                        )));
                }
                _ => {}
            }
            if self.inputs.text_start.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::TextChange(
                        NavTextChange::MoveCursorStart,
                    )));
            }
            if self.inputs.text_end.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::TextChange(
                        NavTextChange::MoveCursorEnd,
                    )));
            }
            if self.inputs.text_delete_left.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::TextChange(
                        NavTextChange::DeleteLeft,
                    )));
            }
            if self.inputs.text_delete_right.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::TextChange(
                        NavTextChange::DeleteRight,
                    )));
            }
            if self.inputs.trigger.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::TextChange(
                        NavTextChange::NewLine,
                    )));
            }
        } else {
            match self.inputs.direction.get() {
                GuiCardinalDirection::Up => {
                    self.engine.interact(Interaction::Navigate(NavSignal::Up));
                }
                GuiCardinalDirection::Down => {
                    self.engine.interact(Interaction::Navigate(NavSignal::Down));
                }
                GuiCardinalDirection::Left => {
                    self.engine.interact(Interaction::Navigate(NavSignal::Left));
                }
                GuiCardinalDirection::Right => {
                    self.engine
                        .interact(Interaction::Navigate(NavSignal::Right));
                }
                _ => {}
            }
            if self.inputs.prev.get().is_pressed() {
                self.engine.interact(Interaction::Navigate(NavSignal::Prev));
            }
            if self.inputs.next.get().is_pressed() {
                self.engine.interact(Interaction::Navigate(NavSignal::Next));
            }
            let trigger = self.inputs.trigger.get();
            if trigger.is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Accept(true)));
            }
            if trigger.is_released() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Accept(false)));
            }
            let context = self.inputs.context.get();
            if context.is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Context(true)));
            }
            if context.is_released() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Context(false)));
            }
            let cancel = self.inputs.cancel.get();
            if cancel.is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Cancel(true)));
            }
            if cancel.is_released() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Cancel(false)));
            }
        }
        let pointer_position = {
            let [x, y] = self.inputs.pointer_position.get();
            let position = mapping.real_to_virtual_vec2(Vec2 { x, y }, false);
            if (position.x - self.cached_pointer_position.x).abs() > ZERO_THRESHOLD
                || (position.y - self.cached_pointer_position.y).abs() > ZERO_THRESHOLD
            {
                self.cached_pointer_position = position;
                self.engine.interact(Interaction::PointerMove(position));
            }
            position
        };
        let pointer_trigger = self.inputs.pointer_trigger.get();
        let pointer_context = self.inputs.pointer_context.get();
        if pointer_trigger.is_pressed() {
            self.engine.interact(Interaction::PointerDown(
                PointerButton::Trigger,
                pointer_position,
            ));
        }
        if pointer_trigger.is_released() {
            self.engine.interact(Interaction::PointerUp(
                PointerButton::Trigger,
                pointer_position,
            ));
        }
        if pointer_context.is_pressed() {
            self.engine.interact(Interaction::PointerDown(
                PointerButton::Context,
                pointer_position,
            ));
        }
        if pointer_context.is_released() {
            self.engine.interact(Interaction::PointerUp(
                PointerButton::Context,
                pointer_position,
            ));
        }
        {
            let [x, y] = self.inputs.scroll.get();
            if x.abs() > ZERO_THRESHOLD || y.abs() > ZERO_THRESHOLD {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Jump(NavJump::Scroll(
                        NavScroll::Units(Vec2 { x: -x, y: -y }, true),
                    ))));
            }
        }
    }
}

impl InteractionsEngine<DefaultInteractionsEngineResult, ()> for GuiInteractionsEngine {
    fn perform_interactions(
        &mut self,
        app: &mut Application,
    ) -> Result<DefaultInteractionsEngineResult, ()> {
        self.engine.perform_interactions(app)
    }
}
