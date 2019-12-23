#![allow(dead_code)]

use std;
use glium;
use glium::Surface;

pub struct GliumDisplayWinitWrapper(pub glium::Display);

impl conrod_winit::WinitWindow for GliumDisplayWinitWrapper {
  fn get_inner_size(&self) -> Option<(u32, u32)> {
    self.0.gl_window().get_inner_size().map(Into::into)
  }
  fn hidpi_factor(&self) -> f32 {
    self.0.gl_window().get_hidpi_factor() as _
  }
}

/// In most of the examples the `glutin` crate is used for providing the window context and
/// events while the `glium` crate is used for displaying `conrod_core::render::Primitives` to the
/// screen.
///
/// This `Iterator`-like type simplifies some of the boilerplate involved in setting up a
/// glutin+glium event loop that works efficiently with conrod.
pub struct EventLoop {
  ui_needs_update: bool,
  last_update: std::time::Instant,
}

impl EventLoop {
  pub fn new() -> Self {
    EventLoop {
      last_update: std::time::Instant::now(),
      ui_needs_update: true,
    }
  }

  /// Produce an iterator yielding all available events.
  pub fn next(&mut self, events_loop: &mut glium::glutin::EventsLoop) -> Vec<glium::glutin::Event> {
    // We don't want to loop any faster than 60 FPS, so wait until it has been at least 16ms
    // since the last yield.
    let last_update = self.last_update;
    let sixteen_ms = std::time::Duration::from_millis(16);
    let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
    if duration_since_last_update < sixteen_ms {
      std::thread::sleep(sixteen_ms - duration_since_last_update);
    }

    // Collect all pending events.
    let mut events = Vec::new();
    events_loop.poll_events(|event| events.push(event));

    // If there are no events and the `Ui` does not need updating, wait for the next event.
    if events.is_empty() && !self.ui_needs_update {
      events_loop.run_forever(|event| {
        events.push(event);
        glium::glutin::ControlFlow::Break
      });
    }

    self.ui_needs_update = false;
    self.last_update = std::time::Instant::now();

    events
  }

  /// Notifies the event loop that the `Ui` requires another update whether or not there are any
/// pending events.
///
/// This is primarily used on the occasion that some part of the `Ui` is still animating and
/// requires further updates to do so.
  pub fn needs_update(&mut self) {
    self.ui_needs_update = true;
  }
}

pub fn render(
  display: glium::Display,
  ui: &mut conrod_core::Ui,
  mut ui_state: super::UIState,
  events_loop: &mut glium::glutin::EventsLoop,
  ui_events: fn(event: glium::glutin::WindowEvent, ui_state: &mut super::UIState),
  widgets: fn(ui: conrod_core::UiCell, ui_state: &mut super::UIState),
) {
  let display = GliumDisplayWinitWrapper(display);

  // A type used for converting `conrod_core::render::Primitives` into `Command`s that can be used
  // for drawing to the glium `Surface`.
  let mut renderer = conrod_glium::Renderer::new(&display.0).unwrap();

  // The image map describing each of our widget->image mappings (in our case, none).
  let image_map = conrod_core::image::Map::<glium::texture::Texture2d>::new();

  let mut events = Vec::new();

  'render: loop {
    events.clear();

    // Get all the new events since the last frame.
    events_loop.poll_events(|event| { events.push(event); });

    // If there are no new events, wait for one.
    if events.is_empty() {
      events_loop.run_forever(|event| {
        events.push(event);
        glium::glutin::ControlFlow::Break
      });
    }

    // Process the events.
    for event in events.drain(..) {

      // Break from the loop upon `Escape` or closed window.
      match event.clone() {
        glium::glutin::Event::WindowEvent { event, .. } => {
          match event {
            glium::glutin::WindowEvent::CloseRequested |
            glium::glutin::WindowEvent::KeyboardInput {
              input: glium::glutin::KeyboardInput {
                virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape),
                ..
              },
              ..
            } => break 'render,
            _ => ui_events(event, &mut ui_state)
          }
        }
        _ => (),
      };

      // Use the `winit` backend feature to convert the winit event to a conrod input.
      let input = match convert_event(event, &display) {
        None => continue,
        Some(input) => input
      };

      // Handle the input with the `Ui`.
      ui.handle_event(input);

      widgets(ui.set_widgets(), &mut ui_state);

      // Draw the `Ui` if it has changed.
      if let Some(primitives) = ui.draw_if_changed() {
        renderer.fill(&display.0, primitives, &image_map);
        let mut target = display.0.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        renderer.draw(&display.0, &mut target, &image_map).unwrap();
        target.finish().unwrap();
      }
    }
  }
}

// Conversion functions for converting between types from glium's version of `winit` and
// `conrod_core`.
conrod_winit::conversion_fns!();

/// A set of reasonable stylistic defaults that works for the `gui` below.
pub fn theme() -> conrod_core::Theme {
  use conrod_core::position::{Align, Direction, Padding, Position, Relative};
  conrod_core::Theme {
    name: "Demo Theme".to_string(),
    padding: Padding::none(),
    x_position: Position::Relative(Relative::Align(Align::Start), None),
    y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
    background_color: conrod_core::color::DARK_CHARCOAL,
    shape_color: conrod_core::color::LIGHT_CHARCOAL,
    border_color: conrod_core::color::BLACK,
    border_width: 0.0,
    label_color: conrod_core::color::WHITE,
    font_id: None,
    font_size_large: 26,
    font_size_medium: 18,
    font_size_small: 12,
    widget_styling: conrod_core::theme::StyleMap::default(),
    mouse_drag_threshold: 0.0,
    double_click_threshold: std::time::Duration::from_millis(500),
  }
}