use engine::gpu::{egui::Egui, Gpu};
use engine::input::InputState;
use engine::{FrameInput, GameState, UpdateResult};
use glam::UVec2;
use std::time::SystemTime;
use winit::event::{VirtualKeyCode as Key, *};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowBuilder};

pub fn hide_cursor(window: &Window, hide: bool) {
    window.set_cursor_visible(!hide);

    let grab_mode = match (hide, cfg!(target_os = "macos")) {
        (false, _) => CursorGrabMode::None,
        (_, true) => CursorGrabMode::Locked,
        (_, false) => CursorGrabMode::Confined,
    };
    _ = window.set_cursor_grab(grab_mode);
}
pub fn toggle_fullscreen(window: &Window) {
    window.set_fullscreen(match window.fullscreen() {
        Some(_) => None,
        None => Some(Fullscreen::Borderless(None)),
    });
}
pub fn win_size(window: &Window) -> UVec2 {
    UVec2::from(<[u32; 2]>::from(window.inner_size()))
}

pub fn main() {
    env_logger::init();

    let mut fps_temp: u32 = 0;
    let mut fps: u32 = 0;
    let mut last_second = SystemTime::now();
    let mut last_frame = SystemTime::now();
    let mut input = InputState::default();
    let mut cursor_hidden = true;

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Voxel Ray Tracing Engine")
        .build(&event_loop)
        .unwrap();
    let mut prev_win_size = win_size(&window);
    hide_cursor(&window, true);

    let gpu = pollster::block_on(Gpu::new(&window));

    let mut egui = Egui::new(&window, &gpu);
    let mut game_state = GameState::new(win_size(&window), gpu);

    event_loop.run(move |event, _, flow| match event {
        e if input.update(&e) => {}
        Event::WindowEvent { event, .. } => match event {
            e if egui.winit.on_event(&egui.ctx, &e).consumed => {}
            WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
            _ => {}
        },
        Event::RedrawRequested(_) => {
            let last_frame_age = SystemTime::now()
                .duration_since(last_frame)
                .unwrap()
                .as_millis();

            if last_frame_age < (1000 / 60) {
                return;
            }
            last_frame = SystemTime::now();
            let win_size = win_size(&window);

            let update_rs = if cursor_hidden {
                game_state.update(&input)
            } else {
                UpdateResult::default()
            };

            if input.key_pressed(Key::T) {
                cursor_hidden = !cursor_hidden;
                hide_cursor(&window, cursor_hidden);
            }
            if input.key_pressed(Key::F) {
                toggle_fullscreen(&window);
            }

            let frame_in = FrameInput {
                fps,
                prev_win_size,
                win_size,
            };
            prev_win_size = win_size;

            let frame_rs = game_state.frame(&window, &update_rs, &frame_in, &input, &mut egui);
            match frame_rs {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => println!("SurfaceError: Lost"),
                Err(wgpu::SurfaceError::OutOfMemory) => *flow = ControlFlow::Exit,
                Err(e) => eprintln!("{e:?}"),
            };

            input.finish_frame();

            fps_temp += 1;
            let now = SystemTime::now();
            if now.duration_since(last_second).unwrap().as_secs() >= 1 {
                last_second = now;
                fps = fps_temp;
                fps_temp = 0;
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}
