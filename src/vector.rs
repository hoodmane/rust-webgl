use wasm_bindgen::prelude::*;


use std::ops::{Add, Sub, Mul, Div, Neg, Deref, AddAssign, SubAssign, MulAssign, DivAssign };

#[wasm_bindgen(inspectable)]
#[derive(Copy, Clone, Debug)]
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
        *self = *self + other;
    }
}


impl SubAssign for Vec3 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Vec4 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self + other;
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


#[derive(Debug, Default)]
pub struct Vec2Buffer {
    backing : Vec<f32>
}

impl Deref for Vec2Buffer {
    type Target = Vec<f32>;
    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}


impl Vec2Buffer {
    pub fn new() -> Self {
        Self {
            backing : Vec::new()
        }
    }

    pub fn push(&mut self, x : f32, y : f32){
        self.backing.push(x);
        self.backing.push(y);
    }

    pub fn push_vec(&mut self, v : Vec2){
        self.backing.push(v.x);
        self.backing.push(v.y);
    }

    pub fn len(&self) -> usize {
        self.backing.len()/2
    }

    pub fn set(&mut self, idx : usize, x : f32, y : f32) {
        self.backing[2*idx] = x;
        self.backing[2*idx + 1] = y;
    }

    pub fn set_vec(&mut self, idx : usize, v : Vec2) {
        self.backing[2*idx] = v.x;
        self.backing[2*idx + 1] = v.y;
    }

    pub fn get(&self, idx : usize) -> Vec2 {
        Vec2 {
            x : self.backing[2*idx],
            y : self.backing[2*idx + 1],
        }
    }

    pub fn clear(&mut self) {
        self.backing.clear();
    }
}

#[derive(Debug, Default)]
pub struct Vec3Buffer {
    backing : Vec<f32>
}

impl Deref for Vec3Buffer {
    type Target = Vec<f32>;
    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}

impl Vec3Buffer {
    pub fn new() -> Self {
        Self {
            backing : Vec::new()
        }
    }

    pub fn push(&mut self, x : f32, y : f32, z : f32){
        self.backing.push(x);
        self.backing.push(y);
        self.backing.push(z);
    }

    pub fn push_vec(&mut self, v : Vec3){
        self.backing.push(v.x);
        self.backing.push(v.y);
        self.backing.push(v.z);
    }

    pub fn get(&self, idx : usize) -> Vec3 {
        Vec3 {
            x : self.backing[3*idx],
            y : self.backing[3*idx + 1],
            z : self.backing[3*idx + 2],
        }
    }

    pub fn set(&mut self, idx : usize, x : f32, y : f32, z : f32) {
        self.backing[3*idx] = x;
        self.backing[3*idx + 1] = y;
        self.backing[3*idx + 2] = z;
    }

    pub fn set_vec(&mut self, idx : usize, v : Vec3) {
        self.backing[3*idx] = v.x;
        self.backing[3*idx + 1] = v.y;
        self.backing[3*idx + 2] = v.z;
    }

    pub fn len(&self) -> usize {
        self.backing.len()/3
    }

    pub fn clear(&mut self) {
        self.backing.clear();
    }

}


#[derive(Debug, Default)]
pub struct Vec4Buffer {
    backing : Vec<f32>
}

impl Deref for Vec4Buffer {
    type Target = Vec<f32>;
    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}

impl Vec4Buffer {
    pub fn new() -> Self {
        Self {
            backing : Vec::new()
        }
    }
    pub fn push(&mut self, x : f32, y : f32, z : f32, w : f32){
        self.backing.push(x);
        self.backing.push(y);
        self.backing.push(z);
        self.backing.push(w);
    }

    pub fn push_vec(&mut self, v : Vec4){
        self.backing.push(v.x);
        self.backing.push(v.y);
        self.backing.push(v.z);
        self.backing.push(v.w);
    }

    pub fn get(&self, idx : usize) -> Vec4 {
        Vec4 {
            x : self.backing[4*idx],
            y : self.backing[4*idx + 1],
            z : self.backing[4*idx + 2],
            w : self.backing[4*idx + 3],
        }
    }

    pub fn set(&mut self, idx : usize, x : f32, y : f32, z : f32, w : f32) {
        self.backing[4*idx] = x;
        self.backing[4*idx + 1] = y;
        self.backing[4*idx + 2] = z;
        self.backing[4*idx + 3] = w;
    }

    pub fn set_vec(&mut self, idx : usize, v : Vec4) {
        self.backing[4*idx] = v.x;
        self.backing[4*idx + 1] = v.y;
        self.backing[4*idx + 2] = v.z;
        self.backing[4*idx + 3] = v.w;
    }

    pub fn len(&self) -> usize {
        self.backing.len() / 4
    }

    pub fn clear(&mut self) {
        self.backing.clear();
    }
}

impl Vec2 {
    pub fn magnitude_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn magnitude(&self) -> f32 {
        f32::sqrt(self.magnitude_sq())
    }

    pub fn normalize(self) -> Self {
        self * (1.0/self.magnitude())
    }
}


impl Vec3 {
    pub fn magnitude_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn magnitude(&self) -> f32 {
        f32::sqrt(self.magnitude_sq())
    }

    pub fn normalize(self) -> Self {
        self * (1.0/self.magnitude())
    }
}

impl Vec4 {
    pub fn magnitude_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    pub fn magnitude(&self) -> f32 {
        f32::sqrt(self.magnitude_sq())
    }

    pub fn normalize(self) -> Self {
        self * (1.0/self.magnitude())
    }
}