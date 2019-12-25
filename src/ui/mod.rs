use orbtk::{prelude::*, render::platform::RenderContext2D, utils};
use term_painter::{ToStyle, Color as TColor};

// OrbTk 2D drawing
#[derive(Clone, Default, PartialEq, Pipeline)]
struct Graphic2DPipeline;

impl render::RenderPipeline for Graphic2DPipeline {
  fn draw(&self, render_target: &mut render::RenderTarget) {
    let width = render_target.width();
    let height = render_target.height();

    let mut render_context =
      RenderContext2D::new(width, height);

    render_context.set_fill_style(utils::Brush::SolidColor(Color::from("#000000")));

    render_context.fill_rect(0.0, 0.0, width, height);
    render_target.draw(render_context.data());
  }
}

pub fn init() {
  // use this only if you want to run it as web application.
  //orbtk::initialize();

  Application::new()
    .window(|ctx| {
      Window::create()
        .title("OpenCL Attractor")
        .position((100.0, 100.0))
        .size(640.0, 480.0 + 46.0)
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
                .on_click(|_, _| {
                  println!("{} button pressed", TColor::Green.paint("ui::evt:"));
                  true
                })
                .build(ctx),
            )
            .child(
              Canvas::create()
                .attach(Grid::row(1))
                .render_pipeline(RenderPipeline(Box::new(Graphic2DPipeline::default())))
                .build(ctx)
            )
            .build(ctx)
        )
        .build(ctx)
    })
    .run();
}
