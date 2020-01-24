#pragma OPENCL EXTENSION cl_khr_global_int32_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_local_int32_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_global_int32_extended_atomics : enable
#pragma OPENCL EXTENSION cl_khr_local_int32_extended_atomics : enable
#pragma OPENCL EXTENSION cl_khr_int64_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_int64_extended_atomics : enable

#include "kernel/complex.cl"

__constant uint MAX_ORBIT_LENGTH = 1024;

/* projection window (pixel) */
__constant complex projection_size = (complex)( 3, 3 );
__constant complex projection_offset = (complex)( -0.5, 0 );

/* screen offset and zoom (crop) */
__constant complex screen_size = (complex)( 1, 1 );
__constant complex screen_center = (complex)( 0, 0 );
__constant float aspectRatio = 1;

/* Enable atomics with global memory (2x slowdown) */
__constant bool SyncWrite = true;

#include "kernel/util.cl"
#include "kernel/draw_image.cl"

#define init \
  complex z = EPSILON_SMALL;

#define loop \
  z = c_powr(z, 2) + pixel;


#define bailout \
  !(isfinite(z.x) & isfinite(z.y))

uint CheckOrbit(complex const pixel){
  init;

  for(int i = 0; i < MAX_ORBIT_LENGTH; i++){
    loop;
       
    if (bailout)
      return i;

    //if (c_abs(z) >= 4.0f )
    //  return i;
    // this is a bit faster to rely just on f32 infinity, avoiding branching
  }
  
  return MAX_ORBIT_LENGTH;
}

__kernel void main(
    __global uint * accumulator, 
    __global uint * frequency_max,
    __private uint2 const image_size,
    __global __read_only uint * iter,
    ulong2 random
  ) 
{
  uint id_x = get_global_id(0);
  uint id_y = get_global_id(1);
  uint dimm_x = get_global_size(0);
  uint dimm_y = get_global_size(1);
  
  ulong gid = id_y * dimm_x + id_x;
  //gid = gid + dimm_x * dimm_y * iter[0];

  complex pixel = coords_Abnormal2Window(LCPNG(random + (ulong2)gid));
  
  uint orbit_length  = CheckOrbit(pixel);

  if (orbit_length == 0)
    return;
  
  if (orbit_length < MAX_ORBIT_LENGTH){
    
    /*complex z = EPSILON_SMALL;
    for(int i = 0; i < orbit_length; i++){
      FORMULA; 
      
      uint2 coords = coords_Window2Screen((z + screen_center) / screen_size, (complex)(image_size.x, image_size.y));
      if(coords_testOverflow(coords, image_size)){
        uint index = coords.y * image_size.x + coords.x;

        atom_add(&accumulator[index], orbit_length);
        atom_max(&frequency_max[0], accumulator[index]);
      }
    }*/
    uint2 coords = coords_Window2Screen((pixel + screen_center) / screen_size, (complex)(image_size.x, image_size.y));
    if(coords_testOverflow(coords, image_size)){
      uint index = coords.y * image_size.x + coords.x;

      atom_inc(&accumulator[index]);
      atom_max(&frequency_max[0], accumulator[index]);
    }
  }
}
