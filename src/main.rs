use std::f32;
use std::time::Instant;

use eyre::Result;
use fluidsim::fluid::Cell;
use fluidsim::{
    fluid::Fluid,
    renderer::{FluidTexture, Renderer},
};
use glam::Vec2;
use winit::event::{ElementState, MouseButton};
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

const WINDOW_SIZE: u32 = 800;
const RESOLUTION: usize = 200;
const BRUSH_RADIUS: f32 = 0.1;
const BRUSH_DENSITY: f32 = 1.0;

async fn run() -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(WINDOW_SIZE, WINDOW_SIZE))
        .build(&event_loop)?;

    let fluid = Fluid::new(0.0, 0.0, RESOLUTION);

    let renderer = Renderer::new(window).await?;

    let mut fluid_texture = FluidTexture::new(fluid, &renderer);

    let mut last_tick = Instant::now();

    let mut cursor_position = Vec2::ZERO;
    let mut cursor_velocity = Vec2::ZERO;
    let mut button_pressed = false;

    event_loop.run(move |event, _, control| {
        let now = Instant::now();
        let delta = now - last_tick;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => control.set_exit(),
                WindowEvent::CursorMoved { position, .. } => {
                    let logical_position = position.to_logical(renderer.window.scale_factor());
                    let normalized_pos = window_to_normalized(logical_position);
                    cursor_velocity = (normalized_pos - cursor_position) / delta.as_secs_f32();
                    cursor_position = normalized_pos;
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    if button == MouseButton::Left {
                        button_pressed = state == ElementState::Pressed;
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                last_tick = now;

                if button_pressed {
                    let cell_radius = (BRUSH_RADIUS * RESOLUTION as f32 / 2.0).ceil() as isize;
                    let (cursor_cell_x, cursor_cell_y) = normalized_to_cell(cursor_position);

                    for i in (cursor_cell_x - cell_radius)..=(cursor_cell_x + cell_radius) {
                        for j in (cursor_cell_y - cell_radius)..=(cursor_cell_y + cell_radius) {
                            let normalized_pos = cell_to_normalized(i, j);
                            if normalized_pos.distance_squared(cursor_position)
                                < BRUSH_RADIUS as f32 * BRUSH_RADIUS as f32
                            {
                                let cell = &mut fluid_texture.fluid[(i, j)];
                                cell.density += BRUSH_DENSITY * delta.as_secs_f32();
                                cell.velocity += cursor_velocity;
                            }
                        }
                    }
                }

                fluid_texture.fluid.step(delta);
                fluid_texture.update(&renderer);
                if let Err(err) = renderer.render(&fluid_texture) {
                    eprintln!("{err}");
                }
            }
            _ => {}
        }
    })
}

fn window_to_normalized(position: LogicalPosition<f32>) -> Vec2 {
    Vec2::new(
        position.x / WINDOW_SIZE as f32 * 2.0 - 1.0,
        -position.y / WINDOW_SIZE as f32 * 2.0 + 1.0,
    )
}

fn cell_to_normalized(i: isize, j: isize) -> Vec2 {
    Vec2::new(i as f32, j as f32) / RESOLUTION as f32 * 2.0 - 1.0
}

fn normalized_to_cell(position: Vec2) -> (isize, isize) {
    (
        ((position.x / 2.0 + 0.5) * RESOLUTION as f32) as isize,
        ((position.y / 2.0 + 0.5) * RESOLUTION as f32) as isize,
    )
}

fn main() {
    futures::executor::block_on(run()).expect("failure");
}
