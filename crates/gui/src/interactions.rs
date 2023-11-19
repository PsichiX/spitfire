use raui_core::prelude::*;
use spitfire_input::{ArrayInputCombinator, InputActionRef, InputCharactersRef};

const ZERO_THRESHOLD: f32 = 1.0e-6;

#[derive(Default)]
pub struct GuiInteractionsInputs {
    pub trigger: InputActionRef,
    pub context: InputActionRef,
    pub cancel: InputActionRef,
    pub left: InputActionRef,
    pub right: InputActionRef,
    pub up: InputActionRef,
    pub down: InputActionRef,
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
            if self.inputs.left.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::TextChange(
                        NavTextChange::MoveCursorLeft,
                    )));
            }
            if self.inputs.right.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::TextChange(
                        NavTextChange::MoveCursorRight,
                    )));
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
            if self.inputs.up.get().is_pressed() {
                self.engine.interact(Interaction::Navigate(NavSignal::Up));
            }
            if self.inputs.down.get().is_pressed() {
                self.engine.interact(Interaction::Navigate(NavSignal::Down));
            }
            if self.inputs.left.get().is_pressed() {
                self.engine.interact(Interaction::Navigate(NavSignal::Left));
            }
            if self.inputs.right.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Right));
            }
            if self.inputs.prev.get().is_pressed() {
                self.engine.interact(Interaction::Navigate(NavSignal::Prev));
            }
            if self.inputs.next.get().is_pressed() {
                self.engine.interact(Interaction::Navigate(NavSignal::Next));
            }
            if self.inputs.trigger.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Accept(true)));
            }
            if self.inputs.context.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Context(true)));
            }
            if self.inputs.cancel.get().is_pressed() {
                self.engine
                    .interact(Interaction::Navigate(NavSignal::Cancel(true)));
            }
        }
        let pointer_position = {
            let [x, y] = self.inputs.pointer_position.get();
            let position = Vec2 { x, y };
            if x.abs() > ZERO_THRESHOLD || y.abs() > ZERO_THRESHOLD {
                self.engine.interact(Interaction::PointerMove(
                    mapping.real_to_virtual_vec2(position, false),
                ));
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
