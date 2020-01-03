typedef uint4 color;

color float1ToARGB(float pixel){
  color result = (color)(min((uint)(pixel * 0xFF), (uint)0xFF));
  result.w = (uint)0xFF;
  return result;
}

color UInt32ToARGB(uint pixel){
  color result = (color)0;
  result.w = (pixel & 0xFF) / (float)0xFF;
  result.z = ((pixel >> 0x08) & 0xFF) / (float)0xFF;
  result.y = ((pixel >> 0x10) & 0xFF) / (float)0xFF;
  result.x = ((pixel >> 0x18) & 0xFF) / (float)0xFF;
  
  return result;
}

uint ARGBToUInt32(color pixel){
  return convert_uint(pixel.w * (float)0xFF) | (convert_uint(pixel.z * (float)0xFF) << 0x08) | (convert_uint(pixel.y * (float)0xFF) << 0x10) | (convert_uint(pixel.x * (float)0xFF) << 0x18);
}


uint2 coords_Window2Screen(complex z, complex size){
  return convert_uint2(((z + windowCenter) / windowSize + (float2)1) / (float2)2 * size);
}

complex coords_Normal2Window(complex z){
  return (z * (float2)2.0 - (float2)1.0) * windowSize - windowCenter;
}

complex coords_Abnormal2Window(uint2 z_abnormal){
  float2 z_normal = convert_float2(z_abnormal) / (float2)(UINT_MAX >> 1);
  return (z_normal - (float2)1.0) * windowSize - windowCenter;
}


bool coords_testOverflow(uint2 pixel, uint2 size){
  return  (pixel.x >= 0) && (pixel.x < size.x) &&
          (pixel.y >= 0) && (pixel.y < size.y);
}

/*
 * linear congruential pseudorandom number generator, 
 * as defined by D. H. Lehmer and described by Donald E. Knuth 
 * in The Art of Computer Programming, Volume 3: Seminumerical Algorithms, section 3.2.1.
 */
uint2 LCPNG(ulong2 seed_) {
  uint2 result;
  ulong seed = seed_.x;
  seed = (seed * 0x5DEECE66DL + 0xBL) & ((1L << 48) - 1);
  result.x = seed >> 16;
  seed = seed ^ seed_.y;
  seed = (seed * 0x5DEECE66DL + 0xBL) & ((1L << 48) - 1);
  result.y = seed >> 16;
  return result;
}

void atom_add_float(volatile global float *source, const float operand) {
  union {
    unsigned int intVal;
    float floatVal;
  } newVal;
  union {
    unsigned int intVal;
    float floatVal;
  } prevVal;

  do {
    prevVal.floatVal = *source;
    newVal.floatVal = prevVal.floatVal + operand;
  } while (atomic_cmpxchg((volatile global unsigned int *)source, prevVal.intVal, newVal.intVal) != prevVal.intVal);
}
void atom_max_float(volatile global float *source, const float operand) {
  union {
    unsigned int intVal;
    float floatVal;
  } newVal;
  union {
    unsigned int intVal;
    float floatVal;
  } prevVal;

  do {
    prevVal.floatVal = *source;
    newVal.floatVal = max(prevVal.floatVal, operand);
  } while (atomic_cmpxchg((volatile global unsigned int *)source, prevVal.intVal, newVal.intVal) != prevVal.intVal);
}
