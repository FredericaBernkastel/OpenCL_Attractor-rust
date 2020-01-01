#![allow(dead_code)]
use std::mem;

pub unsafe fn u32_to_u8(mut vec32: Vec<u32>) -> Vec<u8> {
  let ratio = mem::size_of::<u32>() / mem::size_of::<u8>();
  let length = vec32.len() * ratio;
  let capacity = vec32.capacity() * ratio;
  let ptr = vec32.as_mut_ptr() as *mut u8;
  mem::forget(vec32);
  Vec::from_raw_parts(ptr, length, capacity)
}

pub unsafe fn u8_to_u32(mut vec8: Vec<u8>) -> Vec<u32> {
  let ratio = mem::size_of::<u32>() / mem::size_of::<u8>();
  let length = vec8.len() / ratio;
  let capacity = vec8.capacity() / ratio;
  let ptr = vec8.as_mut_ptr() as *mut u32;
  mem::forget(vec8);
  Vec::from_raw_parts(ptr, length, capacity)
}

pub fn debug<F>(f: F)
    where F: FnOnce() {
  if cfg!(debug_assertions) {
    f();
  }
}