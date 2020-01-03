use std::{
  sync::{Arc, Mutex, mpsc::Sender, mpsc::Receiver},
  time::{Instant, SystemTime, Duration},
  thread::JoinHandle
};
use ocl::{ProQue, Buffer, Image, flags, prm::Uint2, prm::Ulong2, SpatialDims, Queue};
use ocl::enums::{ImageChannelOrder, ImageChannelDataType, MemObjectType};
use term_painter::{ToStyle, Color as TColor};
use indicatif::{ProgressBar, ProgressStyle};
use image;
use rand::{self, Rng};
use crate::lib::debug;
use orbtk::prelude::HashMap;

struct Args {
  accumulator: Buffer<u32>,
  framebuffer: Image<u8>,
  framebuffer_preview: Image<u8>,
  frequency_max: Buffer<u32>,
  iter: Buffer<u32>
}

struct Kernels {
  main: ocl::Kernel,
  draw_image: ocl::Kernel,
}

pub struct KernelWrapper{
  main_que: ProQue,
  kernels: Kernels,
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
  New(/* width */ u32, /* height */ u32),
  Render(/* iterations */ u32, /* dimensions */ Vec<u32>),
  SaveImage,
  GetState,
  Interrupt,
  Recompile
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

  let mut kernel_wrapper = KernelWrapper::new((512, 512)).unwrap();
  let tx2 = tx2.lock().expect("mutex is poisoned");
  let rx1 = rx1.lock().expect("mutex is poisoned");
  tx2.send(ActionResult::Ok).unwrap();

  let mut rng = rand::thread_rng();
  let distribution = rand::distributions::Uniform::new_inclusive(0u64, std::u64::MAX);
  let mut random = [0u64; 2];

  'messages: loop {
    match rx1.recv().unwrap() {

      /*** New ***/
      Action::New(width, height) => {
        state.randgen_offset = 0;
        rng = rand::thread_rng();
        let mut result = ActionResult::Err;
        match KernelWrapper::new((1, 1)) { // prevent memory overflow
          Ok(kernel_wrapper_) => {
            kernel_wrapper = kernel_wrapper_;
            match KernelWrapper::new((width, height)) {
              Ok(kernel_wrapper_) => {
                kernel_wrapper = kernel_wrapper_;
                result = ActionResult::Ok;
              },
              Err(e) => println!("{}", e)
            }
          },
          Err(e) => println!("{}", e)
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
        kernel_wrapper.kernels.main.set_default_global_work_size(dimm);
        let t0 = Instant::now();
        let progress_bar = ProgressBar::new(iterations as u64);
        progress_bar.set_style(ProgressStyle::default_bar()
          .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:cyan/blue}] {percent}% iter #{pos} [{eta}]")
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

          // generate random
          for x in &mut random {
            *x = rng.sample(distribution);
          }

          // render kernel
          if let Err(_) = kernel_wrapper.main(iter + state.randgen_offset, (random[0], random[1])){
            break 'render;
          }
          if iter % 64 == 0 {
            kernel_wrapper.draw_image_preview().unwrap();
          }
          progress_bar.inc(1);
        }
        progress_bar.finish_and_clear();
        state.randgen_offset += iterations;

        //kernel.draw_image().unwrap();
        kernel_wrapper.draw_image_preview().unwrap();
        tx2.send(ActionResult::Ok).unwrap();
        state.rendering = false;

        debug(|| println!("{} {:?}", TColor::BrightBlack.paint("opencl::render::profiling:"), t0.elapsed()));
      },

      /*** SaveImage ***/
      Action::SaveImage => {
        kernel_wrapper.draw_image().unwrap();

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
      },

      /*** Recompile ***/
      Action::Recompile => {
        match kernel_wrapper.recompile() {
          Ok(()) => {
            kernel_wrapper.draw_image_preview().unwrap();
            tx2.send(ActionResult::Ok).unwrap();
          },
          Err(e) => {
            println!("{}", e);
            tx2.send(ActionResult::Err).unwrap();
          }
        }
      }
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

pub fn load_source() -> String {
  let files = [
    "kernel/complex.cl",
    "kernel/util.cl",
    "kernel/draw_image.cl",
    "kernel/main.cl"
  ];
  let files = files.iter().map(|&x| {
    if let Ok(src) = std::fs::read_to_string(x) {
      (x.to_string(), src)
    } else {
      println!("Unable to load {}", x);
      (x.to_string(), "".to_string())
    }
  }).collect::<HashMap<String, String>>();
  let mut result = files.get("kernel/main.cl".into()).unwrap().clone();
  for (path, src) in files.iter() {
    result = result.replace(format!("#include \"{}\"", path).as_str(), src);
  };
  result
}

fn build_buffers(
  queue: Queue,
  image_size: (u32, u32),
  framebuffer: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
  framebuffer_preview: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
) -> ocl::Result<Args> {

  Ok(Args {
    accumulator: Buffer::<u32>::builder()
      .queue(queue.clone())
      .flags(flags::MEM_READ_WRITE)
      .len(image_size.0 * image_size.1)
      .fill_val(0u32)
      .build()?,
    framebuffer: Image::<u8>::builder()
      .channel_order(ImageChannelOrder::Rgba)
      .channel_data_type(ImageChannelDataType::UnsignedInt8)
      .image_type(MemObjectType::Image2d)
      .dims(&image_size)
      .flags(flags::MEM_WRITE_ONLY | flags::MEM_HOST_READ_ONLY | flags::MEM_COPY_HOST_PTR)
      .copy_host_slice(framebuffer)
      .queue(queue.clone())
      .build()?,
    framebuffer_preview: Image::<u8>::builder()
      .channel_order(ImageChannelOrder::Rgba)
      .channel_data_type(ImageChannelDataType::UnsignedInt8)
      .image_type(MemObjectType::Image2d)
      .dims((512, 512))
      .flags(flags::MEM_WRITE_ONLY | flags::MEM_HOST_READ_ONLY | flags::MEM_COPY_HOST_PTR)
      .copy_host_slice(framebuffer_preview)
      .queue(queue.clone())
      .build()?,
    frequency_max:Buffer::<u32>::builder()
      .queue(queue.clone())
      .flags(flags::MEM_READ_WRITE)
      .len(1)
      .fill_val(0u32)
      .build()?,
    iter: Buffer::<u32>::builder()
      .queue(queue.clone())
      .flags(flags::MEM_READ_ONLY)
      .len(1)
      .fill_val(0u32)
      .build()?
  })
}

fn build_kernels(que: &ProQue, args: &Args) -> ocl::Result<Kernels> {
  let image_size = args.framebuffer.dims().to_lens().expect("invalid framebuffer");

  Ok(Kernels {
    main: que.kernel_builder("main")
      .arg(&args.accumulator)
      .arg(&args.frequency_max)
      .arg(Uint2::new(image_size[0] as u32, image_size[1] as u32))
      .arg(&args.iter)
      .arg_named("random", Ulong2::new(0, 0))
      .build()?,
    draw_image: que.kernel_builder("draw_image")
      .global_work_size((512, 512))
      .arg_named("preview",false as u32)
      .arg(&args.accumulator)
      .arg(&args.framebuffer)
      .arg(&args.framebuffer_preview)
      .arg(&args.frequency_max)
      .arg_named("block_id", 0u32)
      .build()?
  })
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

    let main_que = ProQue::builder()
      .src(load_source())
      .device(device)
      .dims((512, 512))
      .build()?;

    let args = build_buffers(
      main_que.queue().clone(),
      image_size,
      &framebuffer,
      &framebuffer_preview
    )?;

    let kernels = build_kernels(&main_que, &args)?;

    unsafe {
      if let (Some(image_buffer), Some(image_buffer_preview))
        = (&mut crate::IMAGE_BUFFER, &mut crate::IMAGE_BUFFER_PREVIEW){
        *image_buffer.lock().expect("mutex is poisoned") = framebuffer;
        *image_buffer_preview.lock().expect("mutex is poisoned") = framebuffer_preview;
      } else {
        crate::IMAGE_BUFFER = Some(Arc::new(Mutex::new(framebuffer)));
        crate::IMAGE_BUFFER_PREVIEW = Some(Arc::new(Mutex::new(framebuffer_preview)));
      }
    }

    Ok(KernelWrapper { main_que, kernels, args, image_size })
  }

  pub fn recompile(&mut self) -> ocl::Result<()>{

    /* Update strategy:
     * 1. compile new Program, migrate Device and Context, build Queue
     * 2. migrate device buffers into new queue
     * 3. rebuild kernels
     * 4. update kernel, program, device, context, and queue references
     */

    let que = ProQue::builder()
      .src(load_source())
      .device(self.main_que.device())
      .context(self.main_que.context().clone())
      .dims((512, 512))
      .build()?;

    self.args.accumulator.set_default_queue(que.queue().clone());
    self.args.framebuffer.set_default_queue(que.queue().clone());
    self.args.framebuffer_preview.set_default_queue(que.queue().clone());
    self.args.frequency_max.set_default_queue(que.queue().clone());
    self.args.iter.set_default_queue(que.queue().clone());

    self.kernels = build_kernels(&que, &self.args)?;
    self.main_que = que;

    Ok(())
  }

  pub fn main(&self, iter: u32, random: (u64, u64)) -> ocl::Result<()> {
    self.args.iter.write(&vec![iter]).enq()?;
    self.kernels.main.set_arg("random", Ulong2::new(random.0, random.1))?;
    unsafe {
      self.kernels.main.enq()?;
    }
    Ok(())
  }

  pub fn draw_image(&self) -> ocl::Result<()> {
    let dimensions;
    match self.kernels.draw_image.default_global_work_size() {
      SpatialDims::Two(d0, d1) => dimensions = (d0, d1),
      _ => {
        panic!("invalid kernel dimensions");
      }
    }

    let blocks_count = (
      (self.image_size.0 as f64 / dimensions.0 as f64).ceil() *
      (self.image_size.1 as f64 / dimensions.1 as f64).ceil()) as u32;

    self.kernels.draw_image.set_arg("preview", false as u32)?;
    for block_id in 0..blocks_count {
      self.kernels.draw_image.set_arg("block_id", block_id)?;
      unsafe {
        self.kernels.draw_image.enq()?;
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
    self.kernels.draw_image.set_arg("preview", true as u32)?;
    self.kernels.draw_image.set_arg("block_id", 0u32)?;
    unsafe {
      self.kernels.draw_image.enq()?;
      if let Some(image_buffer) = &mut crate::IMAGE_BUFFER_PREVIEW {
        let mut image_buffer = image_buffer.lock().expect("mutex is poisoned");
        self.args.framebuffer_preview.read(&mut image_buffer).enq()?;
      }
    }
    Ok(())
  }
}
