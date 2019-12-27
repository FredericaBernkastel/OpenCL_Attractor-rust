// #![windows_subsystem = "windows"]
#![feature(type_ascription)]
mod ui;
mod ui_cli;
mod opencl;

use std::thread;
use std::sync::{mpsc::channel, mpsc::Sender, mpsc::Receiver};
use term_painter::{ToStyle, Color as TColor};

static mut IMAGE_BUFFER: Option<Vec<u32>> = None;
// 2-sided rendezvous channel
static mut TX1: Option<Sender<opencl::Action>> = None;
static mut RX2: Option<Receiver<opencl::ActionResult>> = None;

fn main() {
  print!("OpenCL Attractor v0.1, gui + cli interface\nType \"help\" for help.\n");

  let (tx1, rx1) = channel::<opencl::Action>(); // (thr_ui, thr_cli) -> thr_opencl
  let (tx2, rx2) = channel(); // thr_opencl -> thr_ui
  unsafe {
    TX1 = Some(tx1);
    RX2 = Some(rx2);
    IMAGE_BUFFER = Some(vec![0xFF000000u32; 512 * 512]);
  }

  let _thr_cli = thread::spawn( || {
    ui_cli::init();
  });

  let _thr_opencl = Some(thread::spawn(move || {
    let mut scalar = 0u32;
    use opencl::*;
    let kernel = KernelWrapper::new().unwrap();
    tx2.send(ActionResult::Ok).unwrap();
    loop {
      match rx1.recv().unwrap() {
        opencl::Action::Render => {
          println!("{} executing OpenCL kernel...", TColor::Green.paint("thr_opencl:"));
          if let Ok(()) = kernel.main(scalar){
            scalar += 1;
            tx2.send(ActionResult::Ok).unwrap();
          } else {
            tx2.send(ActionResult::Err).unwrap();
          }
        },
        _ => ()
      }
    }
  }));

  let thr_ui = thread::spawn( move || {
    unsafe {
      if let Some(rx2) = &RX2 {
        rx2.recv().unwrap(); // wait for opencl init
        ui::init();
      }
    }
  });

  thr_ui.join().unwrap();
}