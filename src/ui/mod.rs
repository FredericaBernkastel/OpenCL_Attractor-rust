use orbtk::{prelude::*, render::platform::RenderContext2D, utils};
use term_painter::{ToStyle, Color as TColor};
use std::time::Instant;
use super::opencl;

#[derive(Default, AsAny)]
pub struct MainViewState;

impl State for MainViewState {
  fn update(&mut self, _: &mut Registry, _ctx: &mut widget::Context<'_>) {  }
}

widget!(
  MainView<MainViewState> {
    render_pipeline: RenderPipeline
  }
);

impl Template for MainView {
  fn template(self, id: Entity, ctx: &mut BuildContext) -> Self {
    self.name("MainView")
      .render_pipeline(RenderPipeline(Box::new(Graphic2DPipeline::default())))
      .child(
        Grid::create()
        .rows(
          Rows::create()
            .row(46.0)
            .row("*")
            .build(),
        )
        .child(
          Grid::create()
          .rows(
            Rows::create()
              .row("*")
              .build(),
          )
          .columns(
            Columns::create()
            .column("auto")
            .column(8.0)
            .column("auto")
            .build(),
          )
          .child(
            Button::create()
              .attach(Grid::row(0))
              .attach(Grid::column(0))
              .text("render")
              .margin((8.0, 8.0, 0.0, 0.0))
              .size(100.0, 30.0)
              .on_click(move |_states, _|{
                println!("> render");
                unsafe {
                  if let (Some(tx1), Some(rx2)) = (&super::TX1, &super::RX2) {
                    tx1.send(opencl::Action::Render).unwrap();
                    rx2.recv().unwrap(); // wait for opencl to finish rendering
                  }
                }
                true
              })
              .build(ctx),
          )
          .child(
            Button::create()
              .attach(Grid::row(0))
              .attach(Grid::column(2))
              .text("save image")
              .margin((8.0, 8.0, 0.0, 0.0))
              .size(100.0, 30.0)
              .on_click(move |_states, _|{
                println!("> save_image");
                unsafe {
                  if let (Some(tx1), Some(rx2)) = (&super::TX1, &super::RX2) {
                    tx1.send(opencl::Action::SaveImage).unwrap();
                    rx2.recv().unwrap();
                  }
                }
                true
              })
              .build(ctx),
          )
          .build(ctx)
        )
        .child(
          Canvas::create()
            .attach(Grid::column(0))
            .attach(Grid::column_span(3))
            .attach(Grid::row(1))
            .horizontal_alignment(utils::Alignment::Stretch)
            .render_pipeline(id)
            .build(ctx)
        )
        .build(ctx)
      )
  }
}

// OrbTk 2D drawing
#[derive(Clone, Default, PartialEq, Pipeline)]
pub struct Graphic2DPipeline;

impl render::RenderPipeline for Graphic2DPipeline {
  fn draw(&self, render_target: &mut render::RenderTarget) {
    let t0 = Instant::now();
    let canvas_width = render_target.width();
    let canvas_height = render_target.height();
    let mut render_context =
      RenderContext2D::new(canvas_width, canvas_height);
    render_context.set_fill_style(utils::Brush::SolidColor(Color::from("#000000")));

    unsafe {
      if let Some(image_buffer) = &super::IMAGE_BUFFER_PREVIEW {
        let image_buffer = image_buffer.lock().expect("mutex is poisoned");
        /*let mut image: RgbaImage = ImageBuffer::from_raw(
          image_buffer.width,
          image_buffer.height,
          u32_to_u8(image_buffer.data.clone()))
          .expect("imagebuffer is corrupted");
        imageops::resize(
          &mut image,
          canvas_width as u32,
          canvas_height as u32,
          imageops::Nearest
        );*/
        let image = super::u8_to_u32(image_buffer.clone().into_raw());
        //let image = vec![0xFF000000u32; 512 * 512];
        let image = Image::from_data(
          512, 512, image
        ).expect("imagebuffer is corrupted");
        render_context.draw_image(&image, 0.0, 0.0);

      } else {
        render_context.fill_rect(0.0, 0.0, canvas_width, canvas_height);
      }
    }
    render_target.draw(render_context.data());
    println!("{} {:?}", TColor::Green.paint("ui::render::profiling:"), t0.elapsed());
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
        .child(MainView::create().build(ctx))
        .build(ctx)
    })
    .run();
}
