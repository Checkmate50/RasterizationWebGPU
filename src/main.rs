#![feature(never_type)]

use winit::{
    event_loop::{EventLoop, ControlFlow},
    event::{Event, WindowEvent, ElementState, DeviceEvent},
    window::WindowBuilder,
    dpi::LogicalSize,
};
use anyhow::Result;
use futures::executor::block_on;
use rasterization::context::Context;
use glam::{Vec3, Mat3};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 900;

fn main() -> Result<!> {
    let file_path = if let Some(file_path) = std::env::args().nth(1) {
        file_path
    } else {
        eprintln!("Error: Please provide a path to a glb file that has an associated json file of the same name.");
        std::process::exit(1);
    };

    let event_loop = EventLoop::new();
    let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
    let window = WindowBuilder::new().with_inner_size(size).build(&event_loop)?;
    let mut state = block_on(Context::new(&window, file_path))?;
    let mut clicking = false;
    let mut x_accel = 0.0;
    let mut y_accel = 0.0;
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { event: WindowEvent::MouseInput { state: ElementState::Pressed, .. }, .. } => clicking = true,
            Event::WindowEvent { event: WindowEvent::MouseInput { state: ElementState::Released, .. }, .. } => clicking = false,
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

                if state.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            },
            _ => {},
        }

    })
}

