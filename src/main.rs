#![feature(never_type)]

use winit::{
    event_loop::{EventLoop, ControlFlow},
    event::{Event, WindowEvent, ElementState, DeviceEvent, VirtualKeyCode, KeyboardInput},
    window::Window,
};
use anyhow::Result;
use futures::executor::block_on;
use rasterization::context::Context;
use glam::{Vec3, Mat3};
use std::time::{Instant, Duration};

fn main() -> Result<!> {

    #[cfg(debug_assertions)]
    env_logger::init(); // enable logging for vulkan validation layers

    let file_path = if let Some(file_path) = std::env::args().nth(1) {
        file_path
    } else {
        eprintln!("Error: Please provide a path to a glb file that has an associated json file of the same name.");
        std::process::exit(1);
    };

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop)?;
    let mut state = block_on(Context::new(&window, file_path))?;
    let mut clicking = false;
    let mut x_accel = 0.0;
    let mut y_accel = 0.0;
    // this time stuff is a bit clunky rn, I wonder if there's a more elegant way
    let mut start_time = Instant::now();
    let mut pause_time: Option<Instant> = None;
    let mut elapsed = 0.0;
    event_loop.run(move |event, _, control_flow| {
        if pause_time == None {
            let now = Instant::now();
            if let Some(duration) = now.checked_duration_since(start_time) {
                elapsed = duration.as_secs_f32();
            } else {
                start_time = Instant::now();
                elapsed = 0.0;
            }
        }
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { event: WindowEvent::MouseInput { state: ElementState::Pressed, .. }, .. } => clicking = true,
            Event::WindowEvent { event: WindowEvent::MouseInput { state: ElementState::Released, .. }, .. } => clicking = false,
            Event::WindowEvent { event: WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Space), state: ElementState::Released, .. }, .. }, .. } => {
                if let Some(t) = pause_time {
                    start_time += t.elapsed();
                    pause_time = None;
                } else {
                    pause_time = Some(Instant::now());
                }
            },
            Event::WindowEvent { event: WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::R), state: ElementState::Released, .. }, .. }, .. } => {
                start_time = Instant::now();
                pause_time = None;
            },
            Event::WindowEvent { event: WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Left), state: ElementState::Pressed, .. }, .. }, .. } => {
                start_time += Duration::new(0, 50000000);
                if let Some(t) = pause_time {
                    elapsed = (start_time.elapsed().saturating_sub(t.elapsed())).as_secs_f32();
                }
            },
            Event::WindowEvent { event: WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Right), state: ElementState::Pressed, .. }, .. }, .. } => {
                start_time -= Duration::new(0, 50000000);
                if let Some(t) = pause_time {
                    elapsed = (start_time.elapsed().saturating_sub(t.elapsed())).as_secs_f32();
                }
            },
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta: (x, y) }, .. } if clicking => {
                x_accel = x;
                y_accel = y;
            },

            Event::MainEventsCleared => {
                let mut move_mat = Mat3::from_rotation_y((-x_accel as f32).to_radians());
                let max = Vec3::new(0.0, -y_accel as f32, 0.0);
                let angle = f32::atan2(state.scene.camera.eye.cross(max).length(), state.scene.camera.eye.dot(max));
                if angle < 2.8 {
                    let a = state.scene.camera.eye + state.scene.camera.target;
                    let axis = a.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
                    move_mat = move_mat * Mat3::from_axis_angle(axis.normalize(), (y_accel as f32).to_radians());
                }
                state.scene.camera.update(&state.queue, move_mat);

                x_accel *= 0.95;
                if (-0.01..0.01).contains(&x_accel) {
                    x_accel = 0.0;
                }

                y_accel *= 0.95;
                if (-0.01..0.01).contains(&y_accel) {
                    y_accel = 0.0;
                }

                if state.render(elapsed).is_err() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            },
            _ => {},
        }

    })
}

