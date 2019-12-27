#![allow(dead_code)]
use dialoguer::{theme::CustomPromptCharacterTheme, Input};
use term_painter::{ToStyle, Color};
use super::opencl;

pub fn init() {
  let theme = CustomPromptCharacterTheme::new('>');

  loop {
    if let Ok(input) = Input::with_theme(&theme)
      .interact(): Result<String, _> {

      match input.as_ref() {
        "render" => unsafe {
          if let Some(tx1) = &super::TX1 {
            tx1.send(opencl::Action::Render).unwrap();
          }
        },
        "exit" => std::process::exit(0),
        "help" => println!("{} {}", Color::Green.paint("repl::help:"),
r#"Available commands:
* render: render kernel
* exit: terminate process"#),
        _ => println!("{} {}", Color::BrightRed.paint("repl::err:"), "unknown command")
      }
    }
  }

}