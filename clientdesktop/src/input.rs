use glam::{vec2, Vec2};
use std::collections::HashSet;
use winit::event::*;
use winit::keyboard::PhysicalKey;

pub type Key = winit::keyboard::KeyCode;
pub type MouseButton = winit::event::MouseButton;

#[derive(Default)]
pub struct InputState {
    pub pressed_keys: HashSet<Key>,
    pub down_keys: HashSet<Key>,
    pub pressed_mouse_buttons: HashSet<MouseButton>,
    pub down_mouse_buttons: HashSet<MouseButton>,
    pub cursor_delta: Vec2,
    pub cursor_pos: Vec2,
    pub scroll_delta: Vec2,
}
impl InputState {
    pub fn key_pressed(&self, key: &Key) -> bool {
        self.pressed_keys.contains(&key)
    }
    pub fn key_down(&self, key: &Key) -> bool {
        self.down_keys.contains(&key)
    }
    pub fn left_button_down(&self) -> bool {
        self.down_mouse_buttons.contains(&MouseButton::Left)
    }
    pub fn right_button_down(&self) -> bool {
        self.down_mouse_buttons.contains(&MouseButton::Right)
    }
    pub fn left_button_pressed(&self) -> bool {
        self.pressed_mouse_buttons.contains(&MouseButton::Left)
    }
    pub fn right_button_pressed(&self) -> bool {
        self.pressed_mouse_buttons.contains(&MouseButton::Right)
    }

    pub fn finish_frame(&mut self) {
        self.cursor_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
        self.pressed_keys.clear();
        self.pressed_mouse_buttons.clear();
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(key) = event.physical_key else {
                    return false;
                };
                if event.state == ElementState::Pressed {
                    self.pressed_keys.insert(key.clone());
                    self.down_keys.insert(key);
                } else {
                    self.down_keys.remove(&key);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match state == &ElementState::Pressed {
                    true => {
                        self.pressed_mouse_buttons.insert(*button);
                        self.down_mouse_buttons.insert(*button);
                    }
                    false => {
                        self.down_mouse_buttons.remove(button);
                    }
                };
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match *delta {
                    MouseScrollDelta::PixelDelta(pos) => vec2(pos.x as f32, pos.y as f32),
                    MouseScrollDelta::LineDelta(x, y) => vec2(x, y),
                };
                self.scroll_delta += delta;
            }
            _ => return false,
        }
        false
    }

    pub fn on_device_event(&mut self, event: &DeviceEvent) -> bool {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.cursor_pos.x += delta.0 as f32;
                self.cursor_pos.y += delta.1 as f32;
                self.cursor_delta.x += delta.0 as f32;
                self.cursor_delta.y += delta.1 as f32;
            }
            DeviceEvent::MouseWheel { delta } => {
                let delta = match *delta {
                    MouseScrollDelta::PixelDelta(pos) => vec2(pos.x as f32, pos.y as f32),
                    MouseScrollDelta::LineDelta(x, y) => vec2(x, y),
                };
                self.scroll_delta += delta;
            }
            _ => return false,
        }
        false
    }
}
