#![allow(dead_code)]
use std::{
  sync::{Arc, Mutex, mpsc::Sender, mpsc::Receiver},
  time::{Instant, SystemTime},
  thread::JoinHandle,
  mem
};
use ocl::{ProQue, Buffer, Image, flags};
use ocl::enums::{ImageChannelOrder, ImageChannelDataType, MemObjectType};
use term_painter::{ToStyle, Color as TColor};
use image;

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

pub enum Action {
  None,
  Render,
  SaveImage
}

pub enum ActionResult {
  Ok,
  Err
}

pub fn thread(tx2: Sender<ActionResult>, rx1: Receiver<Action>) -> JoinHandle<()> {
  let mut iter = 0u32;
  let kernel = KernelWrapper::new((2048, 2048)).unwrap();
  tx2.send(ActionResult::Ok).unwrap();
  loop {
    match rx1.recv().unwrap() {
      Action::Render => {
        println!("{} executing OpenCL kernel...", TColor::BrightBlack.paint("opencl::thr:"));
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
        println!("{} {:?}", TColor::BrightBlack.paint("opencl::render::profiling:"), t0.elapsed());
      },
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
      _ => ()
    }
  }
}

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

impl KernelWrapper {
  pub fn new(image_size: (u32, u32)) -> Result<KernelWrapper, ocl::Error> {

    let device = ocl::Device::list(
      ocl::Platform::default(), Some(ocl::flags::DEVICE_TYPE_GPU))?
      .first()
      .expect("No GPU devices found")
      .clone();

    println!("{}", TColor::BrightBlack.paint(format!("opencl::device::info: {}", device.to_string())));

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

    fn uint2(v: (u32, u32)) -> u64 {
      ((v.0 as u64) << 32) | (v.1 as u64)
    }

    let kernel_main = main_que.kernel_builder("main")
      .arg(&args.accumulator)
      .arg(&args.frequency_max)
      .arg(uint2(image_size))
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
    let iter = iter * 64;
    unsafe {
      for iter in iter..(iter + 64) {
        self.args.iter.write(&vec![iter]).enq()?;
        self.kernel_main.enq()?;
      }
    }
    Ok(())
  }

  pub fn draw_image(&self) -> ocl::Result<()> {
    let iter_count = (
      self.image_size.0 as f64 * self.image_size.1 as f64
        / self.kernel_draw_image.default_global_work_size().to_len() as f64)
      .ceil() as u32;
    unsafe {
      for iter in 0..iter_count {
        self.args.iter.write(&vec![iter]).enq()?;
        self.kernel_draw_image.enq()?;
      }
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
