//! A simple example that demonstrates using conrod within a basic `winit` window loop, using
//! `glium` to render the `conrod_core::render::Primitives` to screen.

extern crate conrod_glium;
extern crate find_folder;
extern crate glium;

mod support;

use std::error::Error;
use conrod_core::{widget, Colorable, Positionable, Widget};
use glium::Surface;

// Generate the widget identifiers.
widget_ids!(struct WidgetIds { text });

struct UIState {
  widget_ids: WidgetIds,
  text: String,
}

fn ui_events(event: glium::glutin::WindowEvent, ui_state: &mut UIState){
  match event {
    glium::glutin::WindowEvent::CursorMoved { device_id: _, position, modifiers: _ } => {
      ui_state.text = position.x.to_string();
    },
    _ => ()
  }
}

fn widgets(ref mut ui: conrod_core::UiCell, ui_state: &UIState) {
  // Set the widgets.

  widget::Text::new(&ui_state.text)
    .middle_of(ui.window)
    .color(conrod_core::color::WHITE)
    .font_size(32)
    .set(ui_state.widget_ids.text, ui);
}

pub fn init() {
  const WIDTH: u32 = 400;
  const HEIGHT: u32 = 200;

  // Build the window.
  let mut events_loop = glium::glutin::EventsLoop::new();
  let window = glium::glutin::WindowBuilder::new()
    .with_title("Hello Conrod!")
    .with_dimensions((WIDTH, HEIGHT).into());
  let context = glium::glutin::ContextBuilder::new()
    .with_vsync(true)
    .with_multisampling(1);
  let display = glium::Display::new(window, context, &events_loop).unwrap();

  // construct our `Ui`.
  let mut ui = conrod_core::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).build();

  let ui_state = UIState {
    text: String::from("0"),
    widget_ids: WidgetIds::new(ui.widget_id_generator())
  };

  if let Err(e) = load_assets(&mut ui) {
    println!("Failed loading asset files:\n{}", e);
  }
  render(display, &mut ui, ui_state, &mut events_loop);
}

fn load_assets (ui: &mut conrod_core::Ui) -> Result<(), Box<dyn Error>> {
  // Add a `Font` to the `Ui`'s `font::Map` from file.
  let assets = find_folder::Search::KidsThenParents(3, 5).for_folder("assets")?;
  let font_path = assets.join("fonts/NotoSans/NotoSans-Regular.ttf");
  ui.fonts.insert_from_file(font_path)?;
  Ok(())
}

fn render(display: glium::Display, ui: &mut conrod_core::Ui, mut ui_state: UIState, events_loop: &mut glium::glutin::EventsLoop) {
  let display = support::GliumDisplayWinitWrapper(display);

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
      let input = match support::convert_event(event, &display) {
        None => continue,
        Some(input) => input,
      };

      // Handle the input with the `Ui`.
      ui.handle_event(input);
    }

    widgets(ui.set_widgets(), &ui_state);

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