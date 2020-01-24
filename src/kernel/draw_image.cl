__kernel void draw_image(
    __private uint const preview,
    __global uint * accumulator,
    __write_only image2d_t framebuffer,
    __write_only image2d_t framebuffer_preview,
    __global uint * frequency_max,
    __private uint const block_id
  )
{
  uint x = get_global_id(0);
  uint y = get_global_id(1);
  uint2 dimensions = (uint2)(get_global_size(0), get_global_size(1));
  uint2 image_size;
  uint2 image_size_full = (uint2)(get_image_width(framebuffer), get_image_height(framebuffer));
  uint2 image_size_preview = (uint2)(get_image_width(framebuffer_preview), get_image_height(framebuffer_preview));
  if(preview)
    image_size = image_size_preview;
  else
    image_size = image_size_full;
  
  uint blocks_x = ceil((float)image_size.x / (float)dimensions.x);
  uint blocks_y = ceil((float)image_size.y / (float)dimensions.y);
  
  x = x + block_id % blocks_x * dimensions.x;
  y = y + block_id / blocks_y * dimensions.y;
  
  int2 pos_in;
  int2 pos_out = (int2)(x, y);
  if (preview)
    pos_in = convert_int2(
      convert_float2((uint2)(x, y))      / 
      convert_float2(image_size_preview) * 
      convert_float2(image_size_full)
    );
  else
    pos_in = (int2)(x, y);
  
  if (
    (pos_out.x >= image_size.x - 1) || 
    (pos_out.y >= image_size.y - 1) ||
    (pos_in.x >= image_size_full.x - 1) ||
    (pos_in.y >= image_size_full.y - 1)
  )
    return;

  
  if(frequency_max[0] > 0){
    __const float exposure = 1.0f;
    __const float shift = -0.0f;
    __const float gamma = 1.0f;
    
    __const float frequency = (float)accumulator[pos_in.y * image_size_full.x + pos_in.x];
    __const float alpha = log(frequency) / log((float)frequency_max[0]);
    
    //image[y * size.x + x] = ARGBToUInt32(HSL2ARGB(ARGB2HSL(UInt32ToARGB(palette[colorIndex])) * (float3)(1,1,min(pow(alpha, 1 / gamma), (float)1)))) | 0xFF000000;
    //image[y * size.x + x] = ARGBToUInt32(UInt32ToARGB(palette[colorIndex]) * min(pow(alpha, 1 / gamma), (float)1)) | 0xFF000000;
    color pixel = float1ToARGB(min((pow(exposure * alpha, 1 / gamma) + shift), 1.0f));
    if (preview)
      write_imageui(framebuffer_preview, pos_out, pixel);
    else
      write_imageui(framebuffer, pos_out, pixel);
  }
}
