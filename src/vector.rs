use wasm_bindgen::prelude::*;


use std::ops::{Add, Sub, Mul, Div, Neg, AddAssign, SubAssign, MulAssign, DivAssign };

#[wasm_bindgen(inspectable)]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vec2 {
    pub x : f32,
    pub y : f32
}

#[wasm_bindgen]
impl Vec2 {
    #[wasm_bindgen(constructor)]
    pub fn new_js(x : f32, y : f32) -> Self {
        Self::new(x, y)
    }
}

impl Vec2 {
    pub const fn new(x : f32, y : f32) -> Self {
        Self {
            x, y
        }
    }

    pub fn direction(theta : f32) -> Self {
        let (y, x) = f32::sin_cos(theta);
        Self {
            x, y
        }
    }
}


#[wasm_bindgen(inspectable)]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vec3 {
    pub x : f32,
    pub y : f32,
    pub z : f32,
}

#[wasm_bindgen]
impl Vec3 {
    #[wasm_bindgen(constructor)]
    pub fn new_js(x : f32, y : f32, z : f32) -> Self {
        Self::new(x, y, z)
    }
}

impl Vec3 {
    pub const fn new(x : f32, y : f32, z : f32) -> Self {
        Self {
            x, y, z
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vec4 {
    pub x : f32,
    pub y : f32,
    pub z : f32,
    pub w : f32
}

#[wasm_bindgen]
impl Vec4 {
    #[wasm_bindgen(constructor)]
    pub fn new_js(x : f32, y : f32, z : f32, w : f32) -> Self {
        Self::new(x, y, z, w)
    }
}

impl Vec4 {
    pub const fn new(x : f32, y : f32, z : f32, w : f32) -> Self {
        Self {
            x, y, z, w
        }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}


impl Add for Vec3 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Add for Vec4 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Sub for Vec4 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
        }
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
            w: self.w * other,
        }
    }
}



impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, other: f32) -> Self::Output {
        Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, other: f32) -> Self::Output {
        Self {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
        }
    }
}

impl Div<f32> for Vec4 {
    type Output = Self;

    fn div(self, other: f32) -> Self::Output {
        Self {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
            w: self.w / other,
        }
    }
}


impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}


impl AddAssign for Vec3 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl AddAssign for Vec4 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}



impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}


impl SubAssign for Vec3 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl SubAssign for Vec4 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, other : f32) {
        *self = *self * other;
    }
}

impl MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, other : f32) {
        *self = *self * other;
    }
}


impl MulAssign<f32> for Vec4 {
    fn mul_assign(&mut self, other : f32) {
        *self = *self * other;
    }
}

impl DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, other : f32) {
        *self = *self / other;
    }
}

impl DivAssign<f32> for Vec3 {
    fn div_assign(&mut self, other : f32) {
        *self = *self / other;
    }
}


impl DivAssign<f32> for Vec4 {
    fn div_assign(&mut self, other : f32) {
        *self = *self / other;
    }
}


impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self * (-1.0)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self * (-1.0)
    }
}

impl Neg for Vec4 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self * (-1.0)
    }
}


impl Vec2 {
    pub fn dot(v1 : Self, v2 : Self) -> f32 {
        v1.x * v2.x + v1.y * v2.y
    }
    
    pub fn cross(v1 : Self, v2 : Self) -> f32 {
        (v1.x * v2.y) - (v1.y * v2.x)
    }
    

    pub fn magnitude_sq(&self) -> f32 {
        Self::dot(*self, *self)
    }

    pub fn magnitude(&self) -> f32 {
        f32::sqrt(self.magnitude_sq())
    }

    pub fn normalize(self) -> Self {
        self * (1.0/self.magnitude())
    }

    pub fn angle(self) -> f32 {
        f32::atan2(self.y, self.x)
    }

    pub fn perp(self) -> Self {
        let Vec2 {x, y} = self;
        Self::new(-y, x)
    }

}


impl Vec3 {
    pub fn dot(v1 : Self, v2 : Self) -> f32 {
        v1.x * v2.x + v1.y * v2.y + v1.z * v2.z
    }

    pub fn magnitude_sq(&self) -> f32 {
        Self::dot(*self, *self)
    }

    pub fn magnitude(&self) -> f32 {
        f32::sqrt(self.magnitude_sq())
    }

    pub fn normalize(self) -> Self {
        self * (1.0/self.magnitude())
    }
}

impl Vec4 {
    pub fn dot(v1 : Self, v2 : Self) -> f32 {
        v1.x * v2.x + v1.y * v2.y + v1.z * v2.z + v1.w * v2.w
    }
    
    pub fn magnitude_sq(&self) -> f32 {
        Self::dot(*self, *self)
    }

    pub fn magnitude(&self) -> f32 {
        f32::sqrt(self.magnitude_sq())
    }

    pub fn normalize(self) -> Self {
        self * (1.0/self.magnitude())
    }
}

// We need to pass WebGl a js_sys::Float32Array as input data for various purposes. These are not modified by WebGl.
// To get a js_sys::Float32Array from rust data we either need to use Float32Array::view(&[f32]) or Float32Array::view_mut_raw(*mut f32, length : usize).
// Both are pretty unsafe because the slice could be dropped or reallocated while the view exists. For some reason, the slice api takes an immutable slice &[f32]
// but the raw pointer api takes a *mut f32. The mutable option seems to make more sense to me. However, we are not planning to ever use the Float32Array view
// to modify the slice, and we want to allow our functions to take an immutable borrow &[f32].
// So we use std::mem::transmute!
pub trait MutPtrF32 {
    unsafe fn mut_ptr_f32(&self) -> *mut f32;

    fn length(&self) -> usize;
}

impl MutPtrF32 for &[Vec2] {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        std::mem::transmute::<_,*mut f32>(self.as_ptr())
    }
    
    fn length(&self) -> usize {
        self.len() * 2
    }
}

impl MutPtrF32 for &[Vec3] {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        std::mem::transmute::<_,*mut f32>(self.as_ptr())
    }
    
    fn length(&self) -> usize {
        self.len() * 3
    }
}

impl MutPtrF32 for &[Vec4] {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        std::mem::transmute::<_,*mut f32>(self.as_ptr())
    }

    fn length(&self) -> usize {
        self.len() * 4
    }
}