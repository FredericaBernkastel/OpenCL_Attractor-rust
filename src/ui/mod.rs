use orbtk::{prelude::*, render::platform::RenderContext2D};
use term_painter::{ToStyle, Color as TColor};
use std::time::Instant;
use super::opencl;

#[derive(Default, AsAny)]
pub struct MainViewState;

impl State for MainViewState {
  fn update(&mut self, _: &mut Registry, ctx: &mut Context<'_>) {  }
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
          Button::create()
            .attach(Grid::row(0))
            .text("render")
            .margin((8.0, 8.0, 8.0, 8.0))
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
          Canvas::create()
            .attach(Grid::row(1))
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
    let width = render_target.width();
    let height = render_target.height();
    let mut render_context =
      RenderContext2D::new(width, height);
    //render_context.set_fill_style(utils::Brush::SolidColor(Color::from("#000000")));
    //render_context.fill_rect(0.0, 0.0, width, height);

    unsafe {
      if let Some(image) = &super::IMAGE_BUFFER {
       let image = Image::from_data(
          512, 512, image.clone()
        ).expect("imagebuffer is corrupted");
        render_context.draw_image(&image, 0.0, 0.0);
        render_target.draw(render_context.data());
      }
    }
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
