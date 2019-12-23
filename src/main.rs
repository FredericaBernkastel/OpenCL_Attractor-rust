// #![windows_subsystem = "windows"]
#![feature(type_ascription)]
#[macro_use] extern crate conrod_core;
mod ui;
mod ui_cli;

use std::thread;

fn main() {
  print!("OpenCL Attractor v0.1, gui + cli interface\nType \"help\" for help.\n");

  thread::spawn(|| {
    ui_cli::init();
  });

  ui::init();
}