#ifndef MATH_CL
#define MATH_CL

//2 component vector to hold the real and imaginary parts of a complex number:
typedef float2 complex;
#define I ((complex)(0.0, 1.0))

__constant float E = 1e-7;
__constant float EPSILON_SMALL = 1e-12;

bool fEqual(float x, float y)
{
  return (x+E > y && x-E < y);
}

/*
 * Return Real (Imaginary) component of complex number:
 */
float real(complex a){
  return a.x;
}
float imag(complex a){
  return a.y;
}

/*
 * Get the modulus of a complex number (its length):
 */
float c_abs(complex z){
  return hypot(z.x, z.y);
}

float c_abs_squared(complex z){
  return z.x * z.x + z.y * z.y;
}

/*
 * Get the argument of a complex number (its angle):
 * http://en.wikipedia.org/wiki/Complex_number#Absolute_value_and_argument
 */
float c_arg(complex a){
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
complex c_mul(complex a, complex b){
  return (complex)( a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

complex c_sqr(complex a){
  return (complex)( a.x * a.x - a.y * a.y, 2 * a.x * a.y);
}

complex c_exp(complex z) {
  return (complex)(exp(z.x) * cos(z.y), exp(z.x) * sin(z.y));
}

complex c_log(complex a) {
  float b = atan2(a.y, a.x);
  if (b > 0.0) b = b - 2.0 * M_PI;
  return (complex)( log(length(a)), b );
}

/*
 * Rising complex number to a complex power
 * https://en.wikipedia.org/wiki/Exponentiation#Powers_of_complex_numbers
 */
complex c_pow(complex z, complex w){
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
complex c_powr(complex z, float w)      
{ 
  float logr = log(hypot(real(z), imag(z))); 
  float logi = atan2(imag(z), real(z)); 
  float x = exp(logr * w); 
  float y = logi * w; 
  
  float cosy; 
  float siny = sincos(y, &cosy); 
  
  return (complex)(x * cosy, x * siny);
}

#endif /* MATH_CL */
