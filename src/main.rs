// #![windows_subsystem = "windows"]
#![feature(type_ascription)]
#[macro_use] extern crate lazy_static;
mod ui;
mod ui_cli;
mod opencl;

use std::thread;
use std::sync::{Mutex, Arc, mpsc::channel, mpsc::Sender, mpsc::Receiver};
use std::cell::RefCell;
use std::time::Instant;
use term_painter::{ToStyle, Color as TColor};
use std::thread::JoinHandle;

static mut thr_opencl: Option<JoinHandle<()>> = None;
static mut image_buffer: Option<Vec<u32>> = None;
static mut tx1: Option<Sender<opencl::Action>> = None;
static mut rx2: Option<Receiver<()>> = None;

fn main() {
  print!("OpenCL Attractor v0.1, gui + cli interface\nType \"help\" for help.\n");

  let (tx1_, rx1) = channel::<opencl::Action>();
  let (tx2, rx2_) = channel::<()>();
  unsafe {
    tx1 = Some(tx1_);
    rx2 = Some(rx2_);
  }

  let thr_cli = thread::spawn( || {
    ui_cli::init();
  });

  unsafe {
    thr_opencl = Some(thread::spawn(move || {
      let mut scalar = 1;
      let mut kernel = opencl::KernelWrapper::new().unwrap();
      loop{
        let action = rx1.recv().unwrap();
        println!("{} executing OpenCL kernel...", TColor::Green.paint("ui::render:"));
        match action {
          opencl::Action::Action1 => {
            kernel.main(scalar).unwrap();
            scalar <<= 1;
          },
          _ => ()
        }
        tx2.send(()).unwrap();
      }
    }));
  }

  let _thr_ui = thread::spawn( move || {
    ui::init();
  });

  _thr_ui.join().unwrap();
}