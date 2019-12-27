use ocl::{ProQue, Buffer, flags};
use term_painter::{ToStyle, Color as TColor};
use std::time::Instant;

pub struct KernelWrapper{
  kernel: ocl::Kernel,
  buffer: ocl::Buffer<u32>,
  scalar: ocl::Buffer<u32>,
}

impl KernelWrapper {
  pub fn new() -> Result<KernelWrapper, ocl::Error> {
    let src = r#"
      __kernel void main(__global uint* buffer, __global uint* scalar) {
        uint x = get_global_id(0);
        uint y = get_global_id(1);
        uint dimm_x = get_global_size(0);
        uint dimm_y = get_global_size(1);

        buffer[y * dimm_y + x] = 0xFF000000 | (x + y) * scalar[0];
      }
    "#;

    let pro_que = ProQue::builder()
      .src(src)
      .dims((512, 512))
      .build()?;

    let buffer = pro_que.create_buffer::<u32>()?;
    let scalar = Buffer::<u32>::builder()
      .queue(pro_que.queue().clone())
      .flags(flags::MEM_READ_WRITE)
      .len(1)
      .fill_val(0u32)
      .build()?;

    let kernel = pro_que.kernel_builder("main")
      .arg(&buffer)
      .arg(&scalar)
      .build()?;

    Ok(KernelWrapper { kernel, buffer, scalar })
  }

  pub fn main(&self, scalar: u32) -> ocl::Result<()> {
    let t0 = Instant::now();
    self.scalar.write(&vec![scalar]).enq()?;
    unsafe {
      self.kernel.enq()?;
      if let Some(image_buffer) = &mut super::IMAGE_BUFFER {
        self.buffer.read(image_buffer).enq()?;
      }
    }

    println!("{} {:?}", TColor::Green.paint("opencl::render::profiling:"), t0.elapsed());
    Ok(())
  }
}
