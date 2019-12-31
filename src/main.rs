// #![windows_subsystem = "windows"]
#[macro_use] extern crate clap;
mod ui;
mod repl;
mod opencl;

use std::thread;
use std::sync::{mpsc::channel, mpsc::Sender, mpsc::Receiver, Arc, Mutex};
use image;
use term_painter::{ToStyle, Color as TColor};

static mut IMAGE_BUFFER: Option<Arc<Mutex<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>> = None;
static mut IMAGE_BUFFER_PREVIEW: Option<Arc<Mutex<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>> = None;
// 2-sided rendezvous channel
static mut TX1: Option<Sender<opencl::Action>> = None;
static mut RX2: Option<Receiver<opencl::ActionResult>> = None;

fn main() {
  print!("{}\nType \"help\" for help.\n",
         TColor::BrightRed.paint(
           format!("OpenCL Attractor v{}, gui + repl interface", env!("CARGO_PKG_VERSION"))
         )
  );

  let (tx1, rx1) = channel::<opencl::Action>(); // (thr_ui, thr_repl) -> thr_opencl
  let (tx2, rx2) = channel(); // thr_opencl -> (thr_ui, thr_repl)
  unsafe {
    TX1 = Some(tx1);
    RX2 = Some(rx2);
  }

  let _thr_repl = thread::spawn( || {
    repl::init();
  });

  let _thr_opencl = Some(thread::spawn(
    || opencl::thread(tx2, rx1)
  ));

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