use orbtk::prelude::*;

#[derive(Default, AsAny)]
pub struct MainViewState;

impl State for MainViewState {
  fn update(&mut self, _: &mut Registry, ctx: &mut Context<'_>) {
    if let Some(_pipeline) = ctx
      .widget()
      .get_mut::<RenderPipeline>("render_pipeline")
      .0
      .as_any()
      .downcast_ref::<Graphic2DPipeline>()
    { }
  }
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
            .text("Button")
            .margin((8.0, 8.0, 8.0, 8.0))
            .size(100.0, 30.0)
            .on_click(move |_states, _|{
              unsafe {
                if let (Some(tx1), Some(rx2)) = (&super::TX1, &super::RX2) {
                  tx1.send(()).unwrap();
                  rx2.recv().unwrap()
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
    /*let width = render_target.width();
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

    render_target.draw(render_context.data());*/

    unsafe {
      if let Some(image) = &super::IMAGE_BUFFER {
        render_target.draw(image);
      }
    }
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
