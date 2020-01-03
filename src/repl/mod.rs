use std::sync::{Arc, Mutex, mpsc::Sender, mpsc::Receiver};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use term_painter::{ToStyle, Color};
use crate::opencl;

pub fn init(
  tx1_: Arc<Mutex<Sender<opencl::Action>>>,
  rx2_: Arc<Mutex<Receiver<opencl::ActionResult>>>
) {
  // `()` can be used when no completer is required
  let mut rustyline = Editor::<()>::new();

  let mut matches = clap_app!(repl =>
      (@subcommand new =>
        (@arg dimensions: -d --dimensions +takes_value +multiple)
      )
      (@subcommand render =>
        (@arg iter: -i --iter +takes_value)
        (@arg dimensions: -d --dimensions +takes_value +multiple)
      )
      (@subcommand recompile => )
      (@subcommand save_image => )
      (@subcommand help => )
      (@subcommand exit => )
  ).help(
r#"USAGE (repl interface): <command> <opts>

Commmands:
new         new image, clear if existing
  -d, --dimensions=[width height | 512 512] image dimensions

render      render kernel
  -i, --iter=[value | 64]                   iteration count
  -d, --dimensions=[values... | 512 512 1]  worker dimensions

recompile   compile kernel and redraw preview
save_image  save image in current directory
help        print help message
exit        terminate application
"#);

  'repl: loop {
    let readline = rustyline.readline("> ");
    match readline {
      Ok(line) => {
        if let Ok(command) = matches
          .clone()
          .get_matches_from_safe(
            format!("repl {}", line).trim().split(" ")
          ){
          rustyline.add_history_entry(line.as_str());

          let tx1 = tx1_.lock().unwrap();
          let rx2 = rx2_.lock().unwrap();

          match command.subcommand() {

            /*** new ***/
            ("new", Some(command)) => {
              let dimensions = values_t!(command, "dimensions", u32).unwrap_or(vec![512, 512]);
              if dimensions.len() != 2{
                println!("{} {}", Color::BrightRed.paint("repl::err:"), "invalid syntax");
                continue 'repl;
              }
              tx1.send(opencl::Action::New(dimensions[0], dimensions[1])).unwrap();
              rx2.recv().unwrap();
            },

            /*** render ***/
            ("render", Some(command)) => {
              let iter = value_t!(command, "iter", u32).unwrap_or(64);
              let dimensions = values_t!(command, "dimensions", u32).unwrap_or(vec![512, 512]);
              tx1.send(opencl::Action::Render(iter, dimensions)).unwrap();
              //rx2.recv().unwrap();
            },

            /*** recompile ***/
            ("recompile", Some(_)) => {
              tx1.send(opencl::Action::Recompile).unwrap();
              rx2.recv().unwrap();
            },

            /*** save_image ***/
            ("save_image", Some(_)) => {
              tx1.send(opencl::Action::SaveImage).unwrap();
              rx2.recv().unwrap();
            },

            /*** help ***/
            ("help", Some(_)) => {
              matches.print_long_help().ok();
            },

            /*** exit ***/
            ("exit", Some(_)) => {
              if opencl::thread_interrupt(tx1_.clone(), rx2_.clone()){
                std::process::exit(0);
              }
            },

            _ => println!("{} {}", Color::BrightRed.paint("repl::err:"), "unknown command")
          }
        } else {
          println!("{} {}", Color::BrightRed.paint("repl::err:"), "invalid syntax");
        }
      },

      // CTRL-C
      Err(ReadlineError::Interrupted) => {
        if opencl::thread_interrupt(tx1_.clone(), rx2_.clone()){
          std::process::exit(0);
        }
      },
      Err(ReadlineError::Eof) => {
        break 'repl
      },
      Err(err) => {
        println!("{} {:?}", Color::BrightRed.paint("repl::err:"), err);
        break 'repl
      }
    }
  }
}