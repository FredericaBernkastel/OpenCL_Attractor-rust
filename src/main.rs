// #![windows_subsystem = "windows"]
#![feature(type_ascription)]
/*#[macro_use]*/ extern crate orbtk;
mod ui;
mod ui_cli;
mod opencl;

use std::thread;
use std::sync::mpsc::{self, TryRecvError};

fn main() {
  print!("OpenCL Attractor v0.1, gui + cli interface\nType \"help\" for help.\n");

  //let (tx, rx) = mpsc::channel();

  let thr_cli = thread::spawn( || {
    ui_cli::init();
  });
  let thr_gui = thread::spawn( || {
    ui::init();
  });
  let image = opencl::trivial().unwrap();
  thr_cli.join().unwrap();
}