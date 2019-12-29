#![allow(dead_code)]
use dialoguer::{theme::CustomPromptCharacterTheme, Input};
use term_painter::{ToStyle, Color};
use super::opencl;

pub fn init() {
  let theme = CustomPromptCharacterTheme::new('>');

  loop {
    if let Ok(input) = Input::with_theme(&theme)
      .interact(): Result<String, _> {

      unsafe {
        if let (Some(tx1), Some(rx2)) = (&super::TX1, &super::RX2) {
          match input.as_ref() {
            "render" => {
              tx1.send(opencl::Action::Render).unwrap();
              rx2.recv().unwrap();
            },
            "save_image" => {
              tx1.send(opencl::Action::SaveImage).unwrap();
              rx2.recv().unwrap();
            },
            "exit" => std::process::exit(0),
            "help" => println!("{} {}", Color::Green.paint("repl::help:"),
r#"Available commands:
* render: render kernel
* save_image
* exit: terminate process"#),
            _ => println!("{} {}", Color::BrightRed.paint("repl::err:"), "unknown command")
          }
        }
      }
    }
  }

}