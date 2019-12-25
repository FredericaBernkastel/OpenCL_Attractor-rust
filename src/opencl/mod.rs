use ocl::ProQue;

pub fn trivial() -> ocl::Result<Vec<u32>> {
  let src = r#"
    __kernel void main(__global uint* buffer, uint scalar) {
      uint x = get_global_id(0);
      uint y = get_global_id(1);
      uint dimm_x = get_global_size(0);
      uint dimm_y = get_global_size(1);

      buffer[y * dimm_y + x] = x + y;
    }
  "#;

  let pro_que = ProQue::builder()
    .src(src)
    .dims((256, 256))
    .build()?;

  let buffer = pro_que.create_buffer::<u32>()?;

  let kernel = pro_que.kernel_builder("main")
    .arg(&buffer)
    .arg(10.0f32)
    .build()?;

  unsafe { kernel.enq()?; }

  let mut vec: Vec<u32> = vec![0; buffer.len()];
  buffer.read(&mut vec).enq()?;

  println!("The value at index [{}] is now '{}'!", 256 * 1 + 1, vec[256 * 1 + 1]);
  Ok(vec)
}