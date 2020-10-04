use wasm_bindgen::prelude::*;

use lyon::geom::math::{Point, Vector};

use derive_more::{From, Add, Sub, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign, Sum};

use std::convert::From;



#[wasm_bindgen(inspectable)]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct JsPoint {
    pub x : f32,
    pub y : f32
}

impl From<(f32, f32)> for JsPoint {
    fn from((px, py) : (f32, f32)) -> Self {
        Self::new(px, py)
    }
}


impl From<JsPoint> for Point {
    fn from(p : JsPoint) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<JsPoint> for Vector {
    fn from(p : JsPoint) -> Self {
        Self::new(p.x, p.y)
    }
}


impl From<Point> for JsPoint {
    fn from(p : Point) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<Vector> for JsPoint {
    fn from(p : Vector) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<&Point> for JsPoint {
    fn from(p : &Point) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<&Vector> for JsPoint {
    fn from(p : &Vector) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<&JsPoint> for Point {
    fn from(p : &JsPoint) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<&JsPoint> for Vector {
    fn from(p : &JsPoint) -> Self {
        Self::new(p.x, p.y)
    }
}

#[wasm_bindgen]
impl JsPoint {
    #[wasm_bindgen(constructor)]
    pub fn new(x : f32, y : f32) -> Self {
        Self { x, y }
    }
}



#[wasm_bindgen(inspectable)]
#[derive(Copy, Clone, Debug, From, Add, Sub, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign, Sum)]
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
#[derive(Copy, Clone, Debug, From, Add, Sub, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign, Sum)]
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


// impl Vec2 {
//     pub fn dot(v1 : Self, v2 : Self) -> f32 {
//         v1.x * v2.x + v1.y * v2.y
//     }
    
//     pub fn cross(v1 : Self, v2 : Self) -> f32 {
//         (v1.x * v2.y) - (v1.y * v2.x)
//     }
    

//     pub fn magnitude_sq(&self) -> f32 {
//         Self::dot(*self, *self)
//     }

//     pub fn magnitude(&self) -> f32 {
//         f32::sqrt(self.magnitude_sq())
//     }

//     pub fn normalize(self) -> Self {
//         self * (1.0/self.magnitude())
//     }

//     pub fn angle(self) -> f32 {
//         f32::atan2(self.y, self.x)
//     }

//     pub fn perp(self) -> Self {
//         let Vec2 {x, y} = self;
//         Self::new(-y, x)
//     }

// }


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


impl MutPtrF32 for &[Point] {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        self.as_ptr() as *mut f32
    }
    
    fn length(&self) -> usize {
        self.len() * 2
    }
}

impl MutPtrF32 for &[Vector] {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        self.as_ptr() as *mut f32
    }
    
    fn length(&self) -> usize {
        self.len() * 2
    }
}

impl MutPtrF32 for &Vec<Point> {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        self.as_slice().mut_ptr_f32()
    }
    
    fn length(&self) -> usize {
        self.as_slice().length()
    }
}

impl MutPtrF32 for &Vec<Vector> {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        self.as_slice().mut_ptr_f32()
    }
    
    fn length(&self) -> usize {
        self.as_slice().length()
    }
}

pub trait SliceVec2T : MutPtrF32 + IntoIterator {
    // type Item;

    fn len(&self) -> usize;
}

impl SliceVec2T for &[Point] {
    // type Item = Point;
    fn len(&self) -> usize {
        let a : &[_] = self;
        a.len()
    }
}
impl SliceVec2T for &[Vector] {
    // type Item = Vector;
    fn len(&self) -> usize {
        let a : &[_] = self;
        a.len()
    }
}

impl SliceVec2T for &Vec<Point> {
    // type Item = Point;
    fn len(&self) -> usize {
        let a : &[_] = self;
        a.len()
    }
}

impl SliceVec2T for &Vec<Vector> {
    // type Item = Vector;
    fn len(&self) -> usize {
        let a : &[_] = self;
        a.len()
    }
}


// impl MutPtrF32 for &[Vec2] {
//     unsafe fn mut_ptr_f32(&self) -> *mut f32 {
//         std::mem::transmute::<_,*mut f32>(self.as_ptr())
//     }
    
//     fn length(&self) -> usize {
//         self.len() * 2
//     }
// }

impl MutPtrF32 for &[Vec3] {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        self.as_ptr() as *mut f32
    }
    
    fn length(&self) -> usize {
        self.len() * 3
    }
}

impl MutPtrF32 for &[Vec4] {
    unsafe fn mut_ptr_f32(&self) -> *mut f32 {
        self.as_ptr() as *mut f32
    }

    fn length(&self) -> usize {
        self.len() * 4
    }
}

