use glutin::event::{ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use typid::ID;

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
        if self.is_down() {
            truthy
        } else {
            falsy
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct InputAxis(f32);

impl InputAxis {
    pub fn threshold(self, value: f32) -> bool {
        self.0 >= value
    }
}

#[derive(Debug, Default, Clone)]
pub struct InputRef<T: Default + Clone>(Arc<RwLock<T>>);

impl<T: Default + Clone> InputRef<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(RwLock::new(data)))
    }

    pub fn read(&self) -> Option<RwLockReadGuard<T>> {
        self.0.read().ok()
    }

    pub fn write(&self) -> Option<RwLockWriteGuard<T>> {
        self.0.write().ok()
    }

    pub fn get(&self) -> T {
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
    mapper: Box<dyn Fn() -> T>,
}

impl<T: Default> Default for InputCombinator<T> {
    fn default() -> Self {
        Self::new(|| T::default())
    }
}

impl<T> InputCombinator<T> {
    pub fn new(mapper: impl Fn() -> T + 'static) -> Self {
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
        Self(InputCombinator::new(|| {
            std::array::from_fn(|_| Default::default())
        }))
    }
}

impl<const N: usize> ArrayInputCombinator<N> {
    pub fn new(inputs: [impl Into<InputActionOrAxisRef>; N]) -> Self {
        let mut items = std::array::from_fn::<InputActionOrAxisRef, N, _>(|_| Default::default());
        for (index, input) in inputs.into_iter().enumerate() {
            items[index] = input.into();
        }
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
}

impl From<InputMapping> for InputMappingRef {
    fn from(value: InputMapping) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone)]
pub struct InputContext {
    pub mouse_wheel_line_scale: f32,
    /// [(id, mapping)]
    mappings_stack: Vec<(ID<InputMapping>, InputMappingRef)>,
    characters: InputCharactersRef,
}

impl Default for InputContext {
    fn default() -> Self {
        Self {
            mouse_wheel_line_scale: Self::default_mouse_wheel_line_scale(),
            mappings_stack: Default::default(),
            characters: Default::default(),
        }
    }
}

impl InputContext {
    fn default_mouse_wheel_line_scale() -> f32 {
        10.0
    }

    pub fn push_mapping(&mut self, mapping: impl Into<InputMappingRef>) -> ID<InputMapping> {
        let id = ID::default();
        self.mappings_stack.push((id, mapping.into()));
        id
    }

    pub fn pop_mapping(&mut self) -> Option<InputMappingRef> {
        self.mappings_stack.pop().map(|(_, mapping)| mapping)
    }

    pub fn remove_mapping(&mut self, id: ID<InputMapping>) -> Option<InputMappingRef> {
        self.mappings_stack
            .iter()
            .position(|(mid, _)| mid == &id)
            .map(|index| self.mappings_stack.remove(index).1)
    }

    pub fn mapping(&self, id: ID<InputMapping>) -> Option<RwLockReadGuard<InputMapping>> {
        self.mappings_stack
            .iter()
            .find(|(mid, _)| mid == &id)
            .and_then(|(_, mapping)| mapping.read())
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
                    if let VirtualAxis::MouseWheelX | VirtualAxis::MouseWheelY = id {
                        if let Some(mut axis) = axis.write() {
                            axis.0 = 0.0;
                        }
                    }
                }
            }
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
                                if let VirtualAction::KeyButton(button) = id {
                                    if *button == key {
                                        if let Some(mut data) = data.write() {
                                            *data =
                                                data.change(input.state == ElementState::Pressed);
                                            if mapping.consume == InputConsume::Hit {
                                                consume = true;
                                            }
                                        }
                                    }
                                }
                            }
                            for (id, data) in &mapping.axes {
                                if let VirtualAxis::KeyButton(button) = id {
                                    if *button == key {
                                        if let Some(mut data) = data.write() {
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
                            if let VirtualAction::MouseButton(btn) = id {
                                if button == btn {
                                    if let Some(mut data) = data.write() {
                                        *data = data.change(*state == ElementState::Pressed);
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                            }
                        }
                        for (id, data) in &mapping.axes {
                            if let VirtualAxis::MouseButton(btn) = id {
                                if button == btn {
                                    if let Some(mut data) = data.write() {
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
                            if let VirtualAction::Axis(index) = id {
                                if axis == index {
                                    if let Some(mut data) = data.write() {
                                        *data = data.change(value.abs() > 0.5);
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
                                }
                            }
                        }
                        for (id, data) in &mapping.axes {
                            if let VirtualAxis::Axis(index) = id {
                                if axis == index {
                                    if let Some(mut data) = data.write() {
                                        data.0 = *value as _;
                                        if mapping.consume == InputConsume::Hit {
                                            consume = true;
                                        }
                                    }
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
