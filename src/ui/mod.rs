use std::thread;
use std::sync::{Arc, MutexGuard};
use std::sync::{Mutex, mpsc::Receiver};
use std::cell::{Cell, RefCell, Ref, RefMut};
use orbtk::{prelude::*, render::platform::RenderContext2D, utils};
use term_painter::{ToStyle, Color as TColor};

use super::opencl;
use std::thread::JoinHandle;

// OrbTk 2D drawing
#[derive(Clone, Debug, PartialEq, Pipeline)]
pub struct Graphic2DPipeline;

impl render::RenderPipeline for Graphic2DPipeline {
  fn draw(&self, render_target: &mut render::RenderTarget) {
    let width = render_target.width();
    let height = render_target.height();

    let mut render_context =
      RenderContext2D::new(width, height);

    render_context.set_fill_style(utils::Brush::SolidColor(Color::from("#000000")));

    render_context.fill_rect(0.0, 0.0, width, height);

    unsafe {
       if let Some(image) = &super::image_buffer {
        let image = Image::from_data(
          512, 512, image.to_owned()
        ).expect("imagebuffer is corrupted");
        render_context.draw_image(&image, 0.0, 0.0);
      }
    }

    render_target.draw(render_context.data());
  }
}

pub fn init() {
  // use this only if you want to run it as web application.
  //orbtk::initialize();

  Application::new()
    .window(move |ctx| {
      Window::create()
        .title("OpenCL Attractor")
        .position((100.0, 100.0))
        .size(512.0, 512.0 + 46.0)
        .child(
          Grid::create()
          .rows(
            Rows::create()
              .row(46.0)
              .row("*")
              .build(),
          )
          .child(
            Button::create()
              .attach(Grid::row(0))
              .text("Button")
              .margin((8.0, 8.0, 8.0, 8.0))
              .size(100.0, 30.0)
              .on_click(|_, _|{
                unsafe {
                  if let Some(tx1_) = &super::tx1 {
                    tx1_.send(opencl::Action::Action1).unwrap();
                  }
                  if let Some(rx2) = &super::rx2 {
                    rx2.recv().unwrap();
                  }
                }
                true
              })
              .build(ctx),
          )
          .child(
            Canvas::create()
              .attach(Grid::row(1))
              .render_pipeline(RenderPipeline(Box::new(Graphic2DPipeline{})))
              .build(ctx)
          )
          .build(ctx)
        )
        .build(ctx)
    })
    .run();
}
