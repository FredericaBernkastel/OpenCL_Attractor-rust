#pragma OPENCL EXTENSION cl_khr_global_int32_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_local_int32_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_global_int32_extended_atomics : enable
#pragma OPENCL EXTENSION cl_khr_local_int32_extended_atomics : enable
#pragma OPENCL EXTENSION cl_khr_int64_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_int64_extended_atomics : enable

#include "kernel/complex.cl"

__constant uint MAX_ORBIT_LENGTH = 1024;

/* render window (pixel) */
__constant complex windowSize = (complex)( 1.5, 1.5 );
__constant complex windowCenter = (complex)( 0.6, 0 );

/* projection offset and zoom */
__constant complex screenSize = (complex)( 1, 1 );
__constant complex screenCenter = (complex)( 0, 0 );
__constant float aspectRatio = 1;

/* Enable atomics with global memory (2x slowdown) */
__constant bool SyncWrite = true;

#include "kernel/util.cl"
#include "kernel/draw_image.cl"


uint CheckOrbit(complex const c){
  complex z = EPSILON_SMALL;
  
  complex z_period = (complex)(z.x, z.y);
  uint iPeriod = 0;
  uint periodCheckInterval = 3;
  /* __constant */ uint PERIOD_CHECK_MAX = MAX_ORBIT_LENGTH;
  /* __constant */ float PERIOD_CHECK_DELTA = 1e-6f;
  /* __constant */ uint PERIOD_CHECK_INTERVAL = 101;

  int i = 0;
  for(; i < MAX_ORBIT_LENGTH; i++){
    z = c_powr(z, 2.0) + c;
       
    if (!(isfinite(z.x) & isfinite(z.y)))
      return (i == 0) ? 0 : (i-1);

    if (c_abs(z) >= 4.0 )
      return i;   
  }
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

  //complex c = coords_Normal2Window(rand((uint2)(gid, gid >> 32)));
  complex c = coords_Abnormal2Window(LCPNG(random + (ulong2)gid));
  
  uint orbit_length = CheckOrbit(c);
  
  if(orbit_length == 0)
    return;
  
  uint2 coords = coords_Window2Screen((c + screenCenter) / screenSize, (complex)(image_size.x, image_size.y));
  if(coords_testOverflow(coords, image_size)){
    uint index = coords.y * image_size.x + coords.x;
    if(orbit_length < MAX_ORBIT_LENGTH){
      atom_inc(&accumulator[index]);
      atom_max(&frequency_max[0], accumulator[index]);
    }  
  }
}
