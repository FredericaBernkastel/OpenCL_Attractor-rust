#![allow(dead_code)]
use ocl::{ProQue, Buffer, flags};
use term_painter::{ToStyle, Color as TColor};
use std::time::Instant;

pub struct KernelWrapper{
  kernel_main: ocl::Kernel,
  kernel_draw_image: ocl::Kernel,
  accumulator: ocl::Buffer<u32>,
  framebuffer: ocl::Buffer<u32>,
  frequency_max: ocl::Buffer<u32>,
  scalar: ocl::Buffer<u32>,
}

pub enum Action {
  None,
  Render
}

pub enum ActionResult {
  Ok,
  Err
}

impl KernelWrapper {
  pub fn new() -> Result<KernelWrapper, ocl::Error> {

    let device = ocl::Device::list(
      ocl::Platform::default(), Some(ocl::flags::DEVICE_TYPE_GPU))?
      .first()
      .expect("No GPU devices found")
      .clone();

    println!("{} {}", TColor::Green.paint("opencl::device::info:"), device.to_string());

    let src = std::fs::read_to_string("kernel.cl").expect("Unable to load kernel.cl");

    let pro_que = ProQue::builder()
      .src(src)
      .device(device)
      .dims((512, 512))
      .build()?;

    let accumulator = pro_que.create_buffer::<u32>()?;
    let framebuffer = pro_que.create_buffer::<u32>()?;
    let frequency_max = Buffer::<u32>::builder()
      .queue(pro_que.queue().clone())
      .flags(flags::MEM_READ_WRITE)
      .len(1)
      .fill_val(0u32)
      .build()?;
    let scalar = Buffer::<u32>::builder()
      .queue(pro_que.queue().clone())
      .flags(flags::MEM_READ_WRITE)
      .len(1)
      .fill_val(0u32)
      .build()?;

    let kernel_main = pro_que.kernel_builder("main")
      .arg(&accumulator)
      .arg(&framebuffer)
      .arg(&frequency_max)
      .arg(&scalar)
      .build()?;

    let kernel_draw_image = pro_que.kernel_builder("draw_image")
      .arg(&accumulator)
      .arg(&framebuffer)
      .arg(&frequency_max)
      .build()?;

    Ok(KernelWrapper { kernel_main, kernel_draw_image, accumulator, framebuffer, frequency_max, scalar })
  }

  pub fn main(&self, scalar: u32) -> ocl::Result<()> {
    let t0 = Instant::now();
    self.scalar.write(&vec![scalar]).enq()?;
    unsafe {
      self.kernel_main.enq()?;
      self.kernel_draw_image.enq()?;
      if let Some(image_buffer) = &mut super::IMAGE_BUFFER {
        self.framebuffer.read(image_buffer).enq()?;
      }
    }

    println!("{} {:?}", TColor::Green.paint("opencl::render::profiling:"), t0.elapsed());
    Ok(())
  }
}
