use gilrs::{Event as GamepadEvent, EventType as GamepadEventType, Gilrs};
#[cfg(not(target_arch = "wasm32"))]
use glutin::event::{ElementState, MouseScrollDelta, WindowEvent};
use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use typid::ID;
#[cfg(target_arch = "wasm32")]
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};

pub use gilrs::{Axis as GamepadAxis, Button as GamepadButton, GamepadId};
#[cfg(not(target_arch = "wasm32"))]
pub use glutin::event::{MouseButton, VirtualKeyCode};
#[cfg(target_arch = "wasm32")]
pub use winit::event::{MouseButton, VirtualKeyCode};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputConsume {
    #[default]
    None,
    Hit,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualAction {
    KeyButton(VirtualKeyCode),
    MouseButton(MouseButton),
    Axis(u32),
    GamepadButton(GamepadButton),
    GamepadAxis(GamepadAxis),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualAxis {
    KeyButton(VirtualKeyCode),
    MousePositionX,
    MousePositionY,
    MouseWheelX,
    MouseWheelY,
    MouseButton(MouseButton),
    Axis(u32),
    GamepadButton(GamepadButton),
    GamepadAxis(GamepadAxis),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    #[default]
    Idle,
    Pressed,
    Hold,
    Released,
}

impl InputAction {
    pub fn change(self, hold: bool) -> Self {
        match (self, hold) {
            (Self::Idle, true) | (Self::Released, true) => Self::Pressed,
            (Self::Pressed, true) => Self::Hold,
            (Self::Pressed, false) | (Self::Hold, false) => Self::Released,
            (Self::Released, false) => Self::Idle,
            _ => self,
        }
    }

    pub fn update(self) -> Self {
        match self {
            Self::Pressed => Self::Hold,
            Self::Released => Self::Idle,
            _ => self,
        }
    }

    pub fn is_idle(self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn is_pressed(self) -> bool {
        matches!(self, Self::Pressed)
    }

    pub fn is_hold(self) -> bool {
        matches!(self, Self::Hold)
    }

    pub fn is_released(self) -> bool {
        matches!(self, Self::Released)
    }

    pub fn is_up(self) -> bool {
        matches!(self, Self::Idle | Self::Released)
    }

    pub fn is_down(self) -> bool {
        matches!(self, Self::Pressed | Self::Hold)
    }

    pub fn is_changing(self) -> bool {
        matches!(self, Self::Pressed | Self::Released)
    }

    pub fn is_continuing(self) -> bool {
        matches!(self, Self::Idle | Self::Hold)
    }

    pub fn to_scalar(self, falsy: f32, truthy: f32) -> f32 {
        if self.is_down() { truthy } else { falsy }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct InputAxis(pub f32);

impl InputAxis {
    pub fn threshold(self, value: f32) -> bool {
        self.0 >= value
    }
}

#[derive(Debug, Default, Clone)]
pub struct InputRef<T: Default>(Arc<RwLock<T>>);

impl<T: Default> InputRef<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(RwLock::new(data)))
    }

    pub fn read(&'_ self) -> Option<RwLockReadGuard<'_, T>> {
        self.0.read().ok()
    }

    pub fn write(&'_ self) -> Option<RwLockWriteGuard<'_, T>> {
        self.0.write().ok()
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.read().map(|value| value.clone()).unwrap_or_default()
    }

    pub fn set(&self, value: T) {
        if let Some(mut data) = self.write() {
            *data = value;
        }
    }
}

pub type InputActionRef = InputRef<InputAction>;
pub type InputAxisRef = InputRef<InputAxis>;
pub type InputCharactersRef = InputRef<InputCharacters>;
pub type InputMappingRef = InputRef<InputMapping>;

#[derive(Debug, Default, Clone)]
pub enum InputActionOrAxisRef {
    #[default]
    None,
    Action(InputActionRef),
    Axis(InputAxisRef),
}

impl InputActionOrAxisRef {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn get_scalar(&self, falsy: f32, truthy: f32) -> f32 {
        match self {
            Self::None => falsy,
            Self::Action(action) => action.get().to_scalar(falsy, truthy),
            Self::Axis(axis) => axis.get().0,
        }
    }

    pub fn threshold(&self, value: f32) -> bool {
        match self {
            Self::None => false,
            Self::Action(action) => action.get().is_down(),
            Self::Axis(axis) => axis.get().threshold(value),
        }
    }
}

impl From<InputActionRef> for InputActionOrAxisRef {
    fn from(value: InputActionRef) -> Self {
        Self::Action(value)
    }
}

impl From<InputAxisRef> for InputActionOrAxisRef {
    fn from(value: InputAxisRef) -> Self {
        Self::Axis(value)
    }
}

pub struct InputCombinator<T> {
    mapper: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T: Default> Default for InputCombinator<T> {
    fn default() -> Self {
        Self::new(|| T::default())
    }
}

impl<T> InputCombinator<T> {
    pub fn new(mapper: impl Fn() -> T + Send + Sync + 'static) -> Self {
        Self {
            mapper: Box::new(mapper),
        }
    }

    pub fn get(&self) -> T {
        (self.mapper)()
    }
}

#[derive(Default)]
pub struct CardinalInputCombinator(InputCombinator<[f32; 2]>);

impl CardinalInputCombinator {
    pub fn new(
        left: impl Into<InputActionOrAxisRef>,
        right: impl Into<InputActionOrAxisRef>,
        up: impl Into<InputActionOrAxisRef>,
        down: impl Into<InputActionOrAxisRef>,
    ) -> Self {
        let left = left.into();
        let right = right.into();
        let up = up.into();
        let down = down.into();
        Self(InputCombinator::new(move || {
            let left = left.get_scalar(0.0, -1.0);
            let right = right.get_scalar(0.0, 1.0);
            let up = up.get_scalar(0.0, -1.0);
            let down = down.get_scalar(0.0, 1.0);
            [left + right, up + down]
        }))
    }

    pub fn get(&self) -> [f32; 2] {
        self.0.get()
    }
}

#[derive(Default)]
pub struct DualInputCombinator(InputCombinator<f32>);

impl DualInputCombinator {
    pub fn new(
        negative: impl Into<InputActionOrAxisRef>,
        positive: impl Into<InputActionOrAxisRef>,
    ) -> Self {
        let negative = negative.into();
        let positive = positive.into();
        Self(InputCombinator::new(move || {
            let negative = negative.get_scalar(0.0, -1.0);
            let positive = positive.get_scalar(0.0, 1.0);
            negative + positive
        }))
    }

    pub fn get(&self) -> f32 {
        self.0.get()
    }
}

pub struct ArrayInputCombinator<const N: usize>(InputCombinator<[f32; N]>);

impl<const N: usize> Default for ArrayInputCombinator<N> {
    fn default() -> Self {
        Self(InputCombinator::new(|| [0.0; N]))
    }
}

impl<const N: usize> ArrayInputCombinator<N> {
    pub fn new(inputs: [impl Into<InputActionOrAxisRef>; N]) -> Self {
        let items: [InputActionOrAxisRef; N] = inputs.map(|input| input.into());
        Self(InputCombinator::new(move || {
            std::array::from_fn(|index| items[index].get_scalar(0.0, 1.0))
        }))
    }

    pub fn get(&self) -> [f32; N] {
        self.0.get()
    }
}

#[derive(Debug, Default, Clone)]
pub struct InputCharacters {
    characters: String,
}

impl InputCharacters {
    pub fn read(&self) -> &str {
        &self.characters
    }

    pub fn write(&mut self) -> &mut String {
        &mut self.characters
    }

    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.characters)
    }
}

#[derive(Debug, Default, Clone)]
pub struct InputMapping {
    pub actions: HashMap<VirtualAction, InputActionRef>,
    pub axes: HashMap<VirtualAxis, InputAxisRef>,
    pub consume: InputConsume,
    pub layer: isize,
    pub name: Cow<'static, str>,
    pub gamepad: Option<GamepadId>,
}

impl InputMapping {
    pub fn action(mut self, id: VirtualAction, action: InputActionRef) -> Self {
        self.actions.insert(id, action);
        self
    }

    pub fn axis(mut self, id: VirtualAxis, axis: InputAxisRef) -> Self {
        self.axes.insert(id, axis);
        self
    }

    pub fn consume(mut self, consume: InputConsume) -> Self {
        self.consume = consume;
        self
    }

    pub fn layer(mut self, value: isize) -> Self {
        self.layer = value;
        self
    }

    pub fn name(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.name = value.into();
        self
    }

    pub fn gamepad(mut self, gamepad: GamepadId) -> Self {
        self.gamepad = Some(gamepad);
        self
    }
}

impl From<InputMapping> for InputMappingRef {
    fn from(value: InputMapping) -> Self {
        Self::new(value)
    }
}

#[derive(Debug)]
pub struct InputContext {
    pub mouse_wheel_line_scale: f32,
    /// [(id, mapping)]
    mappings_stack: Vec<(ID<InputMapping>, InputMappingRef)>,
    characters: InputCharactersRef,
    gamepads: Option<Gilrs>,
}

impl Default for InputContext {
    fn default() -> Self {
        Self {
            mouse_wheel_line_scale: Self::default_mouse_wheel_line_scale(),
            mappings_stack: Default::default(),
            characters: Default::default(),
            gamepads: None,
        }
    }
}

impl Clone for InputContext {
    fn clone(&self) -> Self {
        Self {
            mouse_wheel_line_scale: self.mouse_wheel_line_scale,
            mappings_stack: self.mappings_stack.clone(),
            characters: self.characters.clone(),
            gamepads: None,
        }
    }
}

impl InputContext {
    fn default_mouse_wheel_line_scale() -> f32 {
        10.0
    }

    pub fn with_gamepads(mut self) -> Self {
        self.gamepads = Gilrs::new().ok();
        self
    }

    pub fn with_gamepads_custom(mut self, gamepads: Gilrs) -> Self {
        self.gamepads = Some(gamepads);
        self
    }

    pub fn gamepads(&self) -> Option<&Gilrs> {
        self.gamepads.as_ref()
    }

    pub fn gamepads_mut(&mut self) -> Option<&mut Gilrs> {
        self.gamepads.as_mut()
    }

    pub fn push_mapping(&mut self, mapping: impl Into<InputMappingRef>) -> ID<InputMapping> {
        let mapping = mapping.into();
        let id = ID::default();
        let layer = mapping.read().unwrap().layer;
        let index = self
            .mappings_stack
            .binary_search_by(|(_, mapping)| {
                mapping
                    .read()
                    .unwrap()
                    .layer
                    .cmp(&layer)
                    .then(Ordering::Less)
            })
            .unwrap_or_else(|index| index);
        self.mappings_stack.insert(index, (id, mapping));
        id
    }

    pub fn pop_mapping(&mut self) -> Option<InputMappingRef> {
        self.mappings_stack.pop().map(|(_, mapping)| mapping)
    }

    pub fn top_mapping(&self) -> Option<&InputMappingRef> {
        self.mappings_stack.last().map(|(_, mapping)| mapping)
    }

    pub fn remove_mapping(&mut self, id: ID<InputMapping>) -> Option<InputMappingRef> {
        self.mappings_stack
            .iter()
            .position(|(mid, _)| mid == &id)
            .map(|index| self.mappings_stack.remove(index).1)
    }

    pub fn mapping(&'_ self, id: ID<InputMapping>) -> Option<RwLockReadGuard<'_, InputMapping>> {
        self.mappings_stack
            .iter()
            .find(|(mid, _)| mid == &id)
            .and_then(|(_, mapping)| mapping.read())
    }

    pub fn stack(&self) -> impl Iterator<Item = &InputMappingRef> {
        self.mappings_stack.iter().map(|(_, mapping)| mapping)
    }

    pub fn characters(&self) -> InputCharactersRef {
        self.characters.clone()
    }

    pub fn maintain(&mut self) {
        for (_, mapping) in &mut self.mappings_stack {
            if let Some(mut mapping) = mapping.write() {
                for action in mapping.actions.values_mut() {
                    if let Some(mut action) = action.write() {
                        *action = action.update();
                    }
                }
                for (id, axis) in &mut mapping.axes {
                    if let VirtualAxis::MouseWheelX | VirtualAxis::MouseWheelY = id
                        && let Some(mut axis) = axis.write()
                    {
                        axis.0 = 0.0;
                    }
                }
            }
        }

        if let Some(gamepads) = self.gamepads.as_mut() {
            while let Some(GamepadEvent { id, event, .. }) = gamepads.next_event() {
                match event {
                    GamepadEventType::ButtonPressed(info, ..) => {
                        for (_, mapping) in self.mappings_stack.iter().rev() {
                            if let Some(mapping) = mapping.read() {
                                if !mapping.gamepad.map(|gamepad| gamepad == id).unwrap_or(true) {
                                    continue;
                                }
                                let mut consume = mapping.consume == InputConsume::All;
                                for (id, data) in &mapping.actions {
                                    if let VirtualAction::GamepadButton(button) = id
                                        && *button == info
                                        && let Some(mut data) = data.write()
                                    {
                                        *data = data.change(true);
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                for (id, data) in &mapping.axes {
                                    if let VirtualAxis::GamepadButton(button) = id
                                        && *button == info
                                        && let Some(mut data) = data.write()
                                    {
                                        data.0 = 1.0;
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                if consume {
                                    break;
                                }
                            }
                        }
                    }
                    GamepadEventType::ButtonReleased(info, ..) => {
                        for (_, mapping) in self.mappings_stack.iter().rev() {
                            if let Some(mapping) = mapping.read() {
                                if !mapping.gamepad.map(|gamepad| gamepad == id).unwrap_or(true) {
                                    continue;
                                }
                                let mut consume = mapping.consume == InputConsume::All;
                                for (id, data) in &mapping.actions {
                                    if let VirtualAction::GamepadButton(button) = id
                                        && *button == info
                                        && let Some(mut data) = data.write()
                                    {
                                        *data = data.change(false);
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                for (id, data) in &mapping.axes {
                                    if let VirtualAxis::GamepadButton(button) = id
                                        && *button == info
                                        && let Some(mut data) = data.write()
                                    {
                                        data.0 = 0.0;
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                if consume {
                                    break;
                                }
                            }
                        }
                    }
                    GamepadEventType::AxisChanged(info, value, ..) => {
                        for (_, mapping) in self.mappings_stack.iter().rev() {
                            if let Some(mapping) = mapping.read() {
                                let mut consume = mapping.consume == InputConsume::All;
                                for (id, data) in &mapping.actions {
                                    if let VirtualAction::GamepadAxis(axis) = id
                                        && *axis == info
                                        && let Some(mut data) = data.write()
                                    {
                                        *data = data.change(value > 0.5);
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                for (id, data) in &mapping.axes {
                                    if let VirtualAxis::GamepadAxis(axis) = id
                                        && *axis == info
                                        && let Some(mut data) = data.write()
                                    {
                                        data.0 = value;
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                if consume {
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            gamepads.inc();
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::ReceivedCharacter(character) => {
                if let Some(mut characters) = self.characters.write() {
                    characters.characters.push(*character);
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(key) = input.virtual_keycode {
                    for (_, mapping) in self.mappings_stack.iter().rev() {
                        if let Some(mapping) = mapping.read() {
                            let mut consume = mapping.consume == InputConsume::All;
                            for (id, data) in &mapping.actions {
                                if let VirtualAction::KeyButton(button) = id
                                    && *button == key
                                    && let Some(mut data) = data.write()
                                {
                                    *data = data.change(input.state == ElementState::Pressed);
                                    if mapping.consume == InputConsume::Hit {
                                        consume = true;
                                    }
                                }
                            }
                            for (id, data) in &mapping.axes {
                                if let VirtualAxis::KeyButton(button) = id
                                    && *button == key
                                    && let Some(mut data) = data.write()
                                {
                                    data.0 = if input.state == ElementState::Pressed {
                                        1.0
                                    } else {
                                        0.0
                                    };
                                    if mapping.consume == InputConsume::Hit {
                                        consume = true;
                                    }
                                }
                            }
                            if consume {
                                break;
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                for (_, mapping) in self.mappings_stack.iter().rev() {
                    if let Some(mapping) = mapping.read() {
                        let mut consume = mapping.consume == InputConsume::All;
                        for (id, data) in &mapping.axes {
                            match id {
                                VirtualAxis::MousePositionX => {
                                    if let Some(mut data) = data.write() {
                                        data.0 = position.x as _;
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                VirtualAxis::MousePositionY => {
                                    if let Some(mut data) = data.write() {
                                        data.0 = position.y as _;
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        if consume {
                            break;
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                for (_, mapping) in self.mappings_stack.iter().rev() {
                    if let Some(mapping) = mapping.read() {
                        let mut consume = mapping.consume == InputConsume::All;
                        for (id, data) in &mapping.axes {
                            match id {
                                VirtualAxis::MouseWheelX => {
                                    if let Some(mut data) = data.write() {
                                        data.0 = match delta {
                                            MouseScrollDelta::LineDelta(x, _) => *x,
                                            MouseScrollDelta::PixelDelta(pos) => pos.x as _,
                                        };
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                VirtualAxis::MouseWheelY => {
                                    if let Some(mut data) = data.write() {
                                        data.0 = match delta {
                                            MouseScrollDelta::LineDelta(_, y) => *y,
                                            MouseScrollDelta::PixelDelta(pos) => pos.y as _,
                                        };
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        if consume {
                            break;
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                for (_, mapping) in self.mappings_stack.iter().rev() {
                    if let Some(mapping) = mapping.read() {
                        let mut consume = mapping.consume == InputConsume::All;
                        for (id, data) in &mapping.actions {
                            if let VirtualAction::MouseButton(btn) = id
                                && button == btn
                                && let Some(mut data) = data.write()
                            {
                                *data = data.change(*state == ElementState::Pressed);
                                if mapping.consume == InputConsume::Hit {
                                    consume = true;
                                }
                            }
                        }
                        for (id, data) in &mapping.axes {
                            if let VirtualAxis::MouseButton(btn) = id
                                && button == btn
                                && let Some(mut data) = data.write()
                            {
                                data.0 = if *state == ElementState::Pressed {
                                    1.0
                                } else {
                                    0.0
                                };
                                if mapping.consume == InputConsume::Hit {
                                    consume = true;
                                }
                            }
                        }
                        if consume {
                            break;
                        }
                    }
                }
            }
            WindowEvent::AxisMotion { axis, value, .. } => {
                for (_, mapping) in self.mappings_stack.iter().rev() {
                    if let Some(mapping) = mapping.read() {
                        let mut consume = mapping.consume == InputConsume::All;
                        for (id, data) in &mapping.actions {
                            if let VirtualAction::Axis(index) = id
                                && axis == index
                                && let Some(mut data) = data.write()
                            {
                                *data = data.change(value.abs() > 0.5);
                                if mapping.consume == InputConsume::Hit {
                                    consume = true;
                                }
                            }
                        }
                        for (id, data) in &mapping.axes {
                            if let VirtualAxis::Axis(index) = id
                                && axis == index
                                && let Some(mut data) = data.write()
                            {
                                data.0 = *value as _;
                                if mapping.consume == InputConsume::Hit {
                                    consume = true;
                                }
                            }
                        }
                        if consume {
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack() {
        let mut context = InputContext::default();
        context.push_mapping(InputMapping::default().name("a").layer(0));
        context.push_mapping(InputMapping::default().name("b").layer(0));
        context.push_mapping(InputMapping::default().name("c").layer(0));
        context.push_mapping(InputMapping::default().name("d").layer(-1));
        context.push_mapping(InputMapping::default().name("e").layer(1));
        context.push_mapping(InputMapping::default().name("f").layer(-1));
        context.push_mapping(InputMapping::default().name("g").layer(1));
        context.push_mapping(InputMapping::default().name("h").layer(-2));
        context.push_mapping(InputMapping::default().name("i").layer(-2));
        context.push_mapping(InputMapping::default().name("j").layer(2));
        context.push_mapping(InputMapping::default().name("k").layer(2));

        let provided = context
            .stack()
            .map(|mapping| {
                let mapping = mapping.read().unwrap();
                (mapping.name.as_ref().to_owned(), mapping.layer)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            provided,
            vec![
                ("h".to_owned(), -2),
                ("i".to_owned(), -2),
                ("d".to_owned(), -1),
                ("f".to_owned(), -1),
                ("a".to_owned(), 0),
                ("b".to_owned(), 0),
                ("c".to_owned(), 0),
                ("e".to_owned(), 1),
                ("g".to_owned(), 1),
                ("j".to_owned(), 2),
                ("k".to_owned(), 2),
            ]
        );
    }
}
