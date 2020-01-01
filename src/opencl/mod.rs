#![allow(dead_code)]
use std::{
  sync::{Arc, Mutex, mpsc::Sender, mpsc::Receiver},
  time::{Instant, SystemTime, Duration},
  thread::JoinHandle
};
use ocl::{ProQue, Buffer, Image, flags, prm::Uint2};
use ocl::enums::{ImageChannelOrder, ImageChannelDataType, MemObjectType};
use term_painter::{ToStyle, Color as TColor};
use indicatif::{ProgressBar, ProgressStyle};
use image;
use crate::lib::debug;

struct Args {
  accumulator: Buffer<u32>,
  framebuffer: Image<u8>,
  framebuffer_preview: Image<u8>,
  frequency_max: Buffer<u32>,
  iter: Buffer<u32>,
}

pub struct KernelWrapper{
  kernel_main: ocl::Kernel,
  kernel_draw_image: ocl::Kernel,
  kernel_draw_image_preview: ocl::Kernel,
  args: Args,
  pub image_size: (u32, u32)
}

#[derive(Clone, PartialEq)]
pub struct ThreadState {
  pub randgen_offset: u32,
  pub rendering: bool
}

#[derive(PartialEq)]
pub enum Action {
  None,
  New(/* width */ u32, /* height */ u32),
  Render(/* iterations */ u32, /* dimensions */ Vec<u32>),
  SaveImage,
  GetState,
  Interrupt
}

#[derive(PartialEq)]
pub enum ActionResult {
  Ok,
  State(ThreadState),
  Err
}

pub fn thread(tx2: Arc<Mutex<Sender<ActionResult>>>, rx1: Arc<Mutex<Receiver<Action>>>) -> JoinHandle<()> {
  let mut state = ThreadState {
    randgen_offset: 0u32,
    rendering: false
  };

  let mut kernel = KernelWrapper::new((512, 512)).unwrap();
  let tx2 = tx2.lock().expect("mutex is poisoned");
  let rx1 = rx1.lock().expect("mutex is poisoned");
  tx2.send(ActionResult::Ok).unwrap();

  'messages: loop {
    match rx1.recv().unwrap() {

      /*** New ***/
      Action::New(width, height) => {
        let mut result = ActionResult::Err;
        if let Ok(kernel_) = KernelWrapper::new((1, 1)) { // prevent memory overflow
          kernel = kernel_;
          if let Ok(kernel_) = KernelWrapper::new((width, height)) {
            kernel = kernel_;
            result = ActionResult::Ok;
          }
        }
        tx2.send(result).unwrap();
      },

      /*** Render ***/
      Action::Render(iterations, dimensions) => {
        let dimm: ocl::SpatialDims;
        match dimensions.len() {
          1 => dimm = (dimensions[0]).into(),
          2 => dimm = (dimensions[0], dimensions[1]).into(),
          3 => dimm = (dimensions[0], dimensions[1], dimensions[2]).into(),
          _ => {
            println!("{} {}", TColor::BrightRed.paint("opencl::thr::err"), "invalid number of dimensions");
            tx2.send(ActionResult::Err).unwrap();
            continue 'messages;
          }
        };

        debug(|| println!("{} executing OpenCL kernel...", TColor::BrightBlack.paint("opencl::thr:")));
        // fix ProgressBar bug
        std::thread::sleep(Duration::from_millis(1));
        println!();

        state.rendering = true;
        kernel.kernel_main.set_default_global_work_size(dimm);
        let t0 = Instant::now();
        let progress_bar = ProgressBar::new(iterations as u64);
        progress_bar.set_style(ProgressStyle::default_bar()
          .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:cyan/blue}] {percent}% {msg} [{eta}]")
          .progress_chars("##-"));

        'render: for iter in 0..iterations {

          // event polling during render
          if let Ok(message) = rx1.try_recv(){
            match message {
              Action::GetState => {
                tx2.send(ActionResult::State(state.clone())).unwrap();
              },
              Action::Interrupt => {
                progress_bar.finish_and_clear();
                println!("{} got interrupt signal", TColor::BrightRed.paint("opencl::thr:"));
                break 'render;
              }
              _ => ()
            }
          }

          if let Err(_) = kernel.main(iter + state.randgen_offset){
            break 'render;
          }
          if iter % 64 == 0 {
            kernel.draw_image().unwrap();
            kernel.draw_image_preview().unwrap();
          }
          progress_bar.inc(1);
          progress_bar.set_message(&format!("iter #{}", iter + 1));
        }
        progress_bar.finish_and_clear();
        state.randgen_offset += iterations;

        kernel.draw_image().unwrap();
        kernel.draw_image_preview().unwrap();
        tx2.send(ActionResult::Ok).unwrap();
        state.rendering = false;

        debug(|| println!("{} {:?}", TColor::BrightBlack.paint("opencl::render::profiling:"), t0.elapsed()));
      },

      /*** SaveImage ***/
      Action::SaveImage => {
        unsafe {
          if let Some(image_buffer) = &crate::IMAGE_BUFFER {
            let image_buffer = image_buffer.lock().expect("mutex is poisoned");
            let file_name = format!(
              "opencl_attractor-{}.png",
              SystemTime::now().duration_since(
                SystemTime::UNIX_EPOCH
              ).unwrap().as_millis()
            );
            if let Ok(()) = image_buffer.save(&file_name){
              println!("{} image saved to \"{}\"", TColor::Green.paint("opencl::thr:"), &file_name);
              tx2.send(ActionResult::Ok).unwrap();
            } else {
              println!("{} unable to save image, \"{}\"", TColor::BrightRed.paint("opencl::thr::err:"), &file_name);
              tx2.send(ActionResult::Err).unwrap();
            }
          }
        }
      },

      /*** GetState ***/
      Action::GetState => {
        tx2.send(ActionResult::State(state.clone())).unwrap();
      },

      /*** Interrupt ***/
      Action::Interrupt => {
        tx2.send(ActionResult::Ok).unwrap();
      }
      _ => ()
    }
  }
}

pub fn thread_interrupt(tx1: Arc<Mutex<Sender<Action>>>, rx2: Arc<Mutex<Receiver<ActionResult>>>) -> bool {
  let tx1 = tx1.lock().expect("mutex is poisoned");
  let rx2 = rx2.lock().expect("mutex is poisoned");
  tx1.send(Action::GetState).unwrap();
  let result = rx2.recv().unwrap();
  match result {
    ActionResult::State(thread_state) => {
      if thread_state.rendering {
        tx1.send(Action::Interrupt).unwrap();
        rx2.recv().unwrap();
      } else {
        return true;
      }
    },
    _ => ()
  }

  return false;
}

impl KernelWrapper {
  pub fn new(image_size: (u32, u32)) -> Result<KernelWrapper, ocl::Error> {

    let device = ocl::Device::list(
      ocl::Platform::default(), Some(ocl::flags::DEVICE_TYPE_GPU))?
      .first()
      .expect("No GPU devices found")
      .clone();

    debug(|| println!("{}", TColor::BrightBlack.paint(format!("opencl::device::info: {}", device.to_string()))));

    let framebuffer = image::ImageBuffer::from_fn(
      image_size.0,
      image_size.1,
      |_, _|{
        image::Rgba([0, 0, 0, 0xFF])
      });
    let framebuffer_preview = image::ImageBuffer::from_fn(
      512,
      512,
      |_, _|{
        image::Rgba([0, 0, 0, 0xFF])
      });

    let src = std::fs::read_to_string("kernel.cl").expect("Unable to load kernel.cl");

    let main_que = ProQue::builder()
      .src(src.clone())
      .device(device)
      .dims((512, 512))
      .build()?;

    let args = Args {
      accumulator: Buffer::<u32>::builder()
        .queue(main_que.queue().clone())
        .flags(flags::MEM_READ_WRITE)
        .len(image_size.0 * image_size.1)
        .fill_val(0u32)
        .build()?,
      framebuffer: Image::<u8>::builder()
        .channel_order(ImageChannelOrder::Rgba)
        .channel_data_type(ImageChannelDataType::UnsignedInt8)
        .image_type(MemObjectType::Image2d)
        .dims(&image_size)
        .flags(flags::MEM_READ_WRITE | flags::MEM_HOST_READ_ONLY | flags::MEM_COPY_HOST_PTR)
        .copy_host_slice(&framebuffer)
        .queue(main_que.queue().clone())
        .build()?,
      framebuffer_preview: Image::<u8>::builder()
        .channel_order(ImageChannelOrder::Rgba)
        .channel_data_type(ImageChannelDataType::UnsignedInt8)
        .image_type(MemObjectType::Image2d)
        .dims((512, 512))
        .flags(flags::MEM_WRITE_ONLY | flags::MEM_HOST_READ_ONLY | flags::MEM_COPY_HOST_PTR)
        .copy_host_slice(&framebuffer_preview)
        .queue(main_que.queue().clone())
        .build()?,
      frequency_max:Buffer::<u32>::builder()
        .queue(main_que.queue().clone())
        .flags(flags::MEM_READ_WRITE)
        .len(1)
        .fill_val(0u32)
        .build()?,
      iter: Buffer::<u32>::builder()
        .queue(main_que.queue().clone())
        .flags(flags::MEM_READ_WRITE)
        .len(1)
        .fill_val(0u32)
        .build()?
    };

    let kernel_main = main_que.kernel_builder("main")
      .arg(&args.accumulator)
      .arg(&args.frequency_max)
      .arg(Uint2::new(image_size.0, image_size.1))
      .arg(&args.iter)
      .build()?;

    let kernel_draw_image = main_que.kernel_builder("draw_image")
      .global_work_size((512, 512))
      .arg(&args.accumulator)
      .arg(&args.framebuffer)
      .arg(&args.frequency_max)
      .arg(&args.iter)
      .build()?;

    let kernel_draw_image_preview = main_que.kernel_builder("draw_image_preview")
      .global_work_size((512, 512))
      .arg(&args.framebuffer)
      .arg(&args.framebuffer_preview)
      .build()?;

    unsafe {
      crate::IMAGE_BUFFER = Some(Arc::new(Mutex::new(framebuffer)));
      crate::IMAGE_BUFFER_PREVIEW = Some(Arc::new(Mutex::new(framebuffer_preview)));
    }

    Ok(KernelWrapper { kernel_main, kernel_draw_image, kernel_draw_image_preview, args, image_size })
  }

  pub fn main(&self, iter: u32) -> ocl::Result<()> {
    self.args.iter.write(&vec![iter]).enq()?;
    unsafe {
      self.kernel_main.enq()?;
    }
    Ok(())
  }

  pub fn draw_image(&self) -> ocl::Result<()> {
    let iter_count = (
      self.image_size.0 as f64 * self.image_size.1 as f64
        / self.kernel_draw_image.default_global_work_size().to_len() as f64)
      .ceil() as u32;

    for iter in 0..iter_count {
      self.args.iter.write(&vec![iter]).enq()?;
      unsafe {
        self.kernel_draw_image.enq()?;
      }
    }
    unsafe {
      if let Some(image_buffer) = &mut crate::IMAGE_BUFFER {
        let mut image_buffer = image_buffer.lock().expect("mutex is poisoned");
        self.args.framebuffer.read(&mut image_buffer).enq()?;
      }
    }
    Ok(())
  }

  pub fn draw_image_preview(&self) -> ocl::Result<()> {
    unsafe {
      self.kernel_draw_image_preview.enq()?;
      if let Some(image_buffer) = &mut crate::IMAGE_BUFFER_PREVIEW {
        let mut image_buffer = image_buffer.lock().expect("mutex is poisoned");
        self.args.framebuffer_preview.read(&mut image_buffer).enq()?;
      }
    }
    Ok(())
  }
}
