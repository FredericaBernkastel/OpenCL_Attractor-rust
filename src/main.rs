// #![windows_subsystem = "windows"]
#![feature(type_ascription)]
mod ui;
mod ui_cli;
mod opencl;

use std::thread;
use std::sync::{mpsc::channel, mpsc::Sender, mpsc::Receiver};
use term_painter::{ToStyle, Color as TColor};

static mut IMAGE_BUFFER: Option<Vec<u32>> = None;
static mut TX1: Option<Sender<()>> = None;
static mut RX2: Option<Receiver<()>> = None;

fn main() {
  print!("OpenCL Attractor v0.1, gui + cli interface\nType \"help\" for help.\n");

  let (tx1, rx1) = channel::<()>();
  let (tx2, rx2) = channel::<()>();
  unsafe {
    TX1 = Some(tx1);
    RX2 = Some(rx2);
    IMAGE_BUFFER = Some(vec![0u32; 512 * 512]);
  }

  let thr_cli = thread::spawn( || {
    ui_cli::init();
  });

  let _thr_opencl = Some(thread::spawn(move || {
    let mut scalar = 1;
    let kernel = opencl::KernelWrapper::new().unwrap();
    loop {
      rx1.recv().unwrap();
      println!("{} executing OpenCL kernel...", TColor::Green.paint("ui::render:"));

      kernel.main(scalar).unwrap();
      scalar <<= 1;
      if scalar & 0x01000000 != 0 {
        scalar = 1;
      };

      tx2.send(()).unwrap();
    }
  }));

  let _thr_ui = thread::spawn( move || {
    ui::init();
  });

  thr_cli.join().unwrap();
}