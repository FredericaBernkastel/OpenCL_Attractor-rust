use std::{
  sync::{Arc, Mutex, mpsc::Sender, mpsc::Receiver},
  thread::JoinHandle,
  time::{Instant, SystemTime, Duration},
  cmp::min
};
use super::KernelWrapper;
use term_painter::{ToStyle, Color as TColor};
use indicatif::{ProgressBar, ProgressStyle};
use rand::{self, Rng};
use crate::lib::debug;

#[derive(Clone, PartialEq)]
pub struct ThreadState {
  pub randgen_offset: u32,
  pub rendering: bool,
  preview_render_interval: u32
}

pub enum Action {
  New(/* width */ u32, /* height */ u32),
  Render(/* iterations */ u32, /* dimensions */ Vec<u32>, /* callback */ Option<Box<dyn FnMut() + Send>>),
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
    rendering: false,
    preview_render_interval: 1u32,
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
        state.preview_render_interval = 1;
        rng = rand::thread_rng();
        let mut result = ActionResult::Err;
        match KernelWrapper::new((1, 1)) { // prevent memory overflow
          Ok(kernel_wrapper_) => {
            kernel_wrapper = kernel_wrapper_;
            match KernelWrapper::new((width, height)) {
              Ok(kernel_wrapper_) => {
                kernel_wrapper = kernel_wrapper_;
                redraw_ui();
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
      Action::Render(iterations, dimensions, callback) => {
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

        tx2.send(ActionResult::Ok).unwrap(); // enqueued

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
                tx2.send(ActionResult::Ok).unwrap();
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
          if iter % state.preview_render_interval == 0 || iter == iterations - 1 {
            kernel_wrapper.draw_image_preview().unwrap();
            redraw_ui();
            state.preview_render_interval = min((state.preview_render_interval as f32 * 1.5).ceil() as u32, 128);
          }
          progress_bar.inc(1);
        }
        progress_bar.finish_and_clear();
        state.randgen_offset += iterations;

        state.rendering = false;
        if let Some(mut callback) = callback {
          callback();
        }

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
            redraw_ui();
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

fn redraw_ui(){
  unsafe {
    if let Some(ui_event) = &mut crate::TX3 {
      ui_event
        .lock()
        .expect("mutex is poisoned")
        .send(orbtk::shell::ShellRequest::Update)
        .unwrap();
    }
  }
}