#pragma OPENCL EXTENSION cl_khr_global_int32_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_local_int32_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_global_int32_extended_atomics : enable
#pragma OPENCL EXTENSION cl_khr_local_int32_extended_atomics : enable
#pragma OPENCL EXTENSION cl_khr_int64_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_int64_extended_atomics : enable

//2 component vector to hold the real and imaginary parts of a complex number:
typedef float2 complex;
#define I ((complex)(0.0, 1.0))

__constant float E = .0000001f;
__constant float EPSILON_SMALL = 1e-12;

bool fEqual(float x, float y)
{
  return (x+E > y && x-E < y);
}

__constant float aspectRatio = 1;

/* render window (pixel) */
__constant complex windowSize = (complex)( 1.5, 1.5 );
__constant complex windowCenter = (complex)( 0.6, 0 );

/* projection offset and zoom */
__constant complex screenSize = (complex)( 1, 1 );
__constant complex screenCenter = (complex)( 0, 0 );

/* Enable atomics with global memory (2x slowdown) */
__constant bool SyncWrite = true;

__constant uint MAX_ORBIT_LENGTH = 32;

typedef float4 color;

inline color UInt32ToARGB(uint pixel){
  color result;
  result.w = (pixel & 0xFF) / (float)0xFF;
  result.z = ((pixel >> 0x08) & 0xFF) / (float)0xFF;
  result.y = ((pixel >> 0x10) & 0xFF) / (float)0xFF;
  result.x = ((pixel >> 0x18) & 0xFF) / (float)0xFF;
  
  return result;
}

inline uint ARGBToUInt32(color pixel){
  return convert_uint(pixel.w * (float)0xFF) | (convert_uint(pixel.z * (float)0xFF) << 0x08) | (convert_uint(pixel.y * (float)0xFF) << 0x10) | (convert_uint(pixel.x * (float)0xFF) << 0x18);
}

/*
 * Return Real (Imaginary) component of complex number:
 */
inline float  real(complex a){
  return a.x;
}
inline float  imag(complex a){
  return a.y;
}

/*
 * Get the modulus of a complex number (its length):
 */
inline float cabs(complex z){
  return hypot(z.x, z.y);
}

inline float cabs_squared(complex z){
  return z.x * z.x + z.y * z.y;
}

/*
 * Get the argument of a complex number (its angle):
 * http://en.wikipedia.org/wiki/Complex_number#Absolute_value_and_argument
 */
inline float carg(complex a){
  if(a.x > 0){
    return atan(a.y / a.x);
  } else if(a.x < 0 && a.y >= 0){
    return atan(a.y / a.x) + M_PI;
  } else if(a.x < 0 && a.y < 0){
    return atan(a.y / a.x) - M_PI;
  } else if(a.x == 0 && a.y > 0){
    return M_PI/2;
  } else if(a.x == 0 && a.y < 0){
    return -M_PI/2;
  } else{
    return 0;
  }
}

/*
 * Multiply two complex numbers:
 */
inline complex cmul(complex a, complex b){
  return (complex)( a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

inline complex csqr(complex a){
  return (complex)( a.x * a.x - a.y * a.y, 2 * a.x * a.y);
}

inline complex cexp(complex z) {
	return (complex)(exp(z.x) * cos(z.y), exp(z.x) * sin(z.y));
}

inline complex clog(complex a) {
	float b = atan2(a.y, a.x);
	if (b > 0.0) b = b - 2.0 * M_PI;
	return (complex)( log(length(a)), b );
}

inline complex cpower2(complex z, complex w) {
	return cexp(cmul(clog(z), w));
}

/*
 * Rising complex number to a complex power
 * https://en.wikipedia.org/wiki/Exponentiation#Powers_of_complex_numbers
 */
inline complex cpow(complex z, complex w){
  float logr = log(hypot(real(z), imag(z)));
  float logi = atan2(imag(z), real(z));

  float x = exp(logr * real(w) - logi * imag(w));
  float y = logr * imag(w) + logi * real(w);
  
  float cosy;
  float siny = sincos(y, &cosy);
  complex result = (complex)(x * cosy, x * siny);
  
  if(isnan(result.x) || isnan(result.y))
    result = HUGE_VALF;
  return result;
}

/*
 * Rising complex number to a real power
 */
inline complex cpowr(complex z, float w)      
{ 
  float logr = log(hypot(real(z), imag(z))); 
  float logi = atan2(imag(z), real(z)); 
  float x = exp(logr * w); 
  float y = logi * w; 
  
  float cosy; 
  float siny = sincos(y, &cosy); 
  
  return (complex)(x * cosy, x * siny);
} 


inline uint2 coords_Window2Screen(complex z, complex size){
  return convert_uint2(((z + windowCenter) / windowSize + (float2)1) / (float2)2 * size);
}

inline complex coords_Normal2Window(complex z){
  return (z * (float2)2.0 - (float2)1.0) * windowSize - windowCenter;
}


inline bool coords_testOverflow(uint2 pixel, uint2 size){
  return  (pixel.x >= 0) && (pixel.x < size.x) &&
          (pixel.y >= 0) && (pixel.y < size.y);
}

inline float2 rand(uint2 state)
{
    const float2 invMaxInt = (float2) (1.0f/4294967296.0f, 1.0f/4294967296.0f);
    uint x = state.x * 17 + state.y * 13123;
    state.x = (x<<13) ^ x;
    state.y ^= (x<<7);

    uint2 tmp = (uint2)
    ( (x * (x * x * 15731 + 74323) + 871483),
      (x * (x * x * 13734 + 37828) + 234234) );

    return convert_float2(tmp) * invMaxInt;
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
    z = cpowr(z, 2.0) + c;
       
		if (!(isfinite(z.x) & isfinite(z.y)))
			return (i == 0) ? 0 : (i-1);
    
    //float _cabs = cabs_squared(z);
    if (cabs(z) >= (4.0f) )
      return i;   
  }
}

__kernel void main(
    __global uint * accumulator, 
    __global uint * framebuffer,
    __global uint * frequency_max,
    __global uint * scalar
  ) 
{
  uint id_x = get_global_id(0);
  uint id_y = get_global_id(1);
  uint dimm_x = get_global_size(0);
  uint dimm_y = get_global_size(1);
  uint2 _size = (uint2)(dimm_x, dimm_y);
  
  //complex c = coords_Normal2Window((complex)(id_x / (float)_size.x, id_y / (float)_size.y));
  complex c = coords_Normal2Window(rand((uint2)(id_x + scalar[0] * dimm_x, id_y + scalar[0] * dimm_y)));
  
  uint orbit_length = CheckOrbit(c);
  
  if(orbit_length == 0)
    return;
  
  uint2 coords = coords_Window2Screen((c + screenCenter) / screenSize, (complex)(_size.x, _size.y));
  if(coords_testOverflow(coords, _size)){
    uint index = coords.y * _size.x + coords.x;
    if(orbit_length < MAX_ORBIT_LENGTH){
      atom_inc(&accumulator[index]);
      atom_max(&frequency_max[0], accumulator[index]);
    }  
  }
  
  //uint alpha = (uint)(sin(x * M_PI * scalar[0]) * sin(y * M_PI * scalar[0]) * 0xFF);
  //buffer[id_y * dimm_y + id_x] = alpha | (alpha << 8) | (alpha << 16) | 0xFF000000;
  //buffer[id_y * dimm_y + id_x] = alpha | 0xFF000000;
}

__kernel void draw_image(
    __global uint * accumulator,
    __global uint * framebuffer,
    __global uint * frequency_max
  )
{
  uint x = get_global_id(0);
  uint y = get_global_id(1);
  uint dimm_x = get_global_size(0);
  uint dimm_y = get_global_size(1);
  uint2 size = (uint2)(dimm_x, dimm_y);
  
  if(frequency_max[0] > 0){
    float frequency = (float)accumulator[y * size.x + x];
    float alpha = 1 * log((frequency + 1)) / log(((float)frequency_max[0] + 1));
    float gamma = 2;
    
    
    //image[y * size.x + x] = ARGBToUInt32(HSL2ARGB(ARGB2HSL(UInt32ToARGB(palette[colorIndex])) * (float3)(1,1,min(pow(alpha, 1 / gamma), (float)1)))) | 0xFF000000;
    //image[y * size.x + x] = ARGBToUInt32(UInt32ToARGB(palette[colorIndex]) * min(pow(alpha, 1 / gamma), (float)1)) | 0xFF000000;
    framebuffer[y * size.x + x] = ARGBToUInt32((color)1 * min(pow(alpha, 1 / gamma), 1.0f)) | 0xFF000000;
  }
}
