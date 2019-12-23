extern crate conrod_glium;
extern crate find_folder;
extern crate glium;

use std::error::Error;
use conrod_core::{widget, Labelable, Colorable, Positionable, Widget};
use glium::glutin::WindowEvent;

mod support;
use conrod_core::position::Sizeable;

// Generate the widget identifiers.
widget_ids!(pub struct WidgetIds { button, button_title, text });

pub struct UIState {
  pub widget_ids: WidgetIds,
  pub text: String
}

fn ui_events(event: WindowEvent, _ui_state: &mut UIState){
  match event {
    /*WindowEvent::CursorMoved { device_id: _, position, modifiers: _ } => {
      ui_state.text = position.x.to_string();
    },*/
    _ => ()
  }
}

fn widgets(ref mut ui: conrod_core::UiCell, ui_state: &mut UIState) {
  // Set the widgets.
  const LABEL_FONT_SIZE: conrod_core::FontSize = 16;
  const MARGIN: f64 = 8f64;

  widget::Text::new(&ui_state.text)
    .middle_of(ui.window)
    .color(conrod_core::color::WHITE)
    .font_size(32)
    .set(ui_state.widget_ids.text, ui);

  for _press in widget::Button::new()
    .label("button")
    .label_font_size(LABEL_FONT_SIZE)
    .top_left_with_margin_on(ui.window, MARGIN)
    .w_h(100f64, 30f64)
    .set(ui_state.widget_ids.button, ui)
  {
    ui_state.text = String::from("pressed");
    println!("ui:: button pressed");
  }
}

pub fn init() {
  const WIDTH: u32 = 400;
  const HEIGHT: u32 = 200;

  // Build the window.
  let mut events_loop = glium::glutin::EventsLoop::new();
  let window = glium::glutin::WindowBuilder::new()
    .with_title("OpenCL Attractor")
    .with_dimensions((WIDTH, HEIGHT).into());
  let context = glium::glutin::ContextBuilder::new()
    .with_vsync(true)
    .with_multisampling(1);
  let display = glium::Display::new(window, context, &events_loop).unwrap();

  // construct our `Ui`.
  let mut ui = conrod_core::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).theme(support::theme()).build();

  let ui_state = UIState {
    text: String::from("0"),
    widget_ids: WidgetIds::new(ui.widget_id_generator())
  };

  if let Err(e) = load_assets(&mut ui) {
    println!("ui:: Failed loading asset files:\n{}", e);
  }
  support::render(display, &mut ui, ui_state, &mut events_loop, ui_events, widgets);
}

fn load_assets (ui: &mut conrod_core::Ui) -> Result<(), Box<dyn Error>> {
  // Add a `Font` to the `Ui`'s `font::Map` from file.
  let assets = find_folder::Search::KidsThenParents(3, 5).for_folder("assets")?;
  let font_path = assets.join("fonts/NotoSans/NotoSans-Regular.ttf");
  ui.fonts.insert_from_file(font_path)?;
  Ok(())
}
