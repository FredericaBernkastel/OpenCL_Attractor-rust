#[allow(dead_code)]
use dialoguer::{theme::CustomPromptCharacterTheme, Input};
use term_painter::{ToStyle, Color};

pub fn init() {
  let theme = CustomPromptCharacterTheme::new('>');

  loop {
    if let Ok(input) = Input::with_theme(&theme)
      .interact(): Result<String, _> {
      println!("{} {}", Color::Green.paint("repl::echo:"), input);
    }
  }

}