// #![windows_subsystem = "windows"]
#[macro_use] extern crate clap;
mod lib;
mod ui;
mod repl;
mod opencl;

use std::thread;
use std::sync::{mpsc::channel, mpsc::Sender, mpsc::Receiver, Arc, Mutex};
use image;
use term_painter::{ToStyle, Color as TColor};

static mut IMAGE_BUFFER: Option<Arc<Mutex<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>> = None;
static mut IMAGE_BUFFER_PREVIEW: Option<Arc<Mutex<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>> = None;
static mut TX1: Option<Arc<Mutex<Sender<opencl::Action>>>> = None;
static mut RX2: Option<Arc<Mutex<Receiver<opencl::ActionResult>>>> = None;

fn main() {
  print!("{}\nType \"help\" for help.\n",
         TColor::BrightRed.paint(
           format!("OpenCL Attractor v{}, gui + repl interface", env!("CARGO_PKG_VERSION"))
         )
  );

  // 4-sided rendezvous channel
  let (tx1, rx1) = channel::<opencl::Action>(); // (thr_ui, thr_repl) -> thr_opencl
  let (tx2, rx2) = channel::<opencl::ActionResult>(); // thr_opencl -> (thr_ui, thr_repl)
  let tx1 = Arc::new(Mutex::new(tx1));
  let rx1 = Arc::new(Mutex::new(rx1));
  let tx2 = Arc::new(Mutex::new(tx2));
  let rx2 = Arc::new(Mutex::new(rx2));
  unsafe {
    TX1 = Some(tx1.clone());
    RX2 = Some(rx2.clone());
  }

  let rx2_ref1 = rx2.clone();
  let _thr_repl = thread::spawn( || {
    repl::init(tx1, rx2_ref1);
  });

  let _thr_opencl = Some(thread::spawn(||
    opencl::thread(tx2, rx1)
  ));

  let thr_ui = thread::spawn( move || {
    {
      let rx2 = rx2.lock().expect("mutex is poisoned");
      rx2.recv().unwrap(); // wait for opencl init
    }
    ui::init();
  });

  thr_ui.join().unwrap();
}