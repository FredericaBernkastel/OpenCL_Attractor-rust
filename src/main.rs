// #![windows_subsystem = "windows"]
#![feature(type_ascription)]
mod ui;
mod ui_cli;
mod opencl;

use std::{
  thread, mem,
  time::{
    SystemTime, Instant
  }
};
use std::sync::{mpsc::channel, mpsc::Sender, mpsc::Receiver, Arc, Mutex};
use term_painter::{ToStyle, Color as TColor};
use image;

static mut IMAGE_BUFFER: Option<Arc<Mutex<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>> = None;
static mut IMAGE_BUFFER_PREVIEW: Option<Arc<Mutex<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>> = None;
// 2-sided rendezvous channel
static mut TX1: Option<Sender<opencl::Action>> = None;
static mut RX2: Option<Receiver<opencl::ActionResult>> = None;

pub unsafe fn u32_to_u8(mut vec32: Vec<u32>) -> Vec<u8> {
  let ratio = mem::size_of::<u32>() / mem::size_of::<u8>();
  let length = vec32.len() * ratio;
  let capacity = vec32.capacity() * ratio;
  let ptr = vec32.as_mut_ptr() as *mut u8;
  mem::forget(vec32);
  Vec::from_raw_parts(ptr, length, capacity)
}

pub unsafe fn u8_to_u32(mut vec8: Vec<u8>) -> Vec<u32> {
  let ratio = mem::size_of::<u32>() / mem::size_of::<u8>();
  let length = vec8.len() / ratio;
  let capacity = vec8.capacity() / ratio;
  let ptr = vec8.as_mut_ptr() as *mut u32;
  mem::forget(vec8);
  Vec::from_raw_parts(ptr, length, capacity)
}

fn main() {
  print!("OpenCL Attractor v0.2, gui + cli interface\nType \"help\" for help.\n");

  let (tx1, rx1) = channel::<opencl::Action>(); // (thr_ui, thr_cli) -> thr_opencl
  let (tx2, rx2) = channel(); // thr_opencl -> thr_ui
  unsafe {
    TX1 = Some(tx1);
    RX2 = Some(rx2);
    //IMAGE_BUFFER = Some(Arc::new(Mutex::new(args.framebuffer)));
    //IMAGE_BUFFER_PREVIEW = Some(Arc::new(Mutex::new()));
  }

  let _thr_cli = thread::spawn( || {
    ui_cli::init();
  });

  let _thr_opencl = Some(thread::spawn(move || {
    let mut iter = 0u32;
    use opencl::*;
    let kernel = KernelWrapper::new((2048, 2048)).unwrap();
    tx2.send(ActionResult::Ok).unwrap();
    loop {
      match rx1.recv().unwrap() {
        Action::Render => {
          println!("{} executing OpenCL kernel...", TColor::Green.paint("thr_opencl:"));
          let t0 = Instant::now();
          kernel.main(iter)
            .and_then(|_| { kernel.draw_image()})
            .and_then(|_| kernel.draw_image_preview())
            .and_then(|_| {
              iter += 1;
              tx2.send(ActionResult::Ok).unwrap();
              Ok(())
            }).unwrap_or_else( |_| {
            tx2.send(ActionResult::Err).unwrap();
          });
          println!("{} {:?}", TColor::Green.paint("opencl::render::profiling:"), t0.elapsed());
        },
        Action::SaveImage => {
          unsafe {
            if let Some(image_buffer) = &IMAGE_BUFFER {
              let image_buffer = image_buffer.lock().expect("mutex is poisoned");
              image_buffer.save(format!(
                "opencl_attractor-{}.png",
                SystemTime::now().duration_since(
                  SystemTime::UNIX_EPOCH
                ).unwrap().as_millis()
              )).expect("unable to save image");
              tx2.send(ActionResult::Ok).unwrap();
            }
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