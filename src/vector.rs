#![allow(dead_code)]

use std::ops::{Add, Sub, Mul, Deref};

#[derive(Copy, Clone, Debug)]
pub struct Vec2<T> {
    pub x : T,
    pub y : T
}

impl<T> Vec2<T> {
    pub const fn new(x : T, y : T) -> Self {
        Self {
            x, y
        }
    }
}

impl Vec2<f32> {
    pub fn direction(theta : f32) -> Self {
        let (y, x) = f32::sin_cos(theta);
        Self {
            x, y
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub struct Vec3<T> {
    pub x : T,
    pub y : T,
    pub z : T,
}

impl<T> Vec3<T> {
    pub const fn new(x : T, y : T, z : T) -> Self {
        Self {
            x, y, z
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Vec4<T> {
    pub x : T,
    pub y : T,
    pub z : T,
    pub w : T
}

impl<T> Vec4<T> {
    pub const fn new(x : T, y : T, z : T, w : T) -> Self {
        Self {
            x, y, z, w
        }
    }
}

impl<T: Add<Output = T>> Add for Vec2<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T: Add<Output = T>> Add for Vec3<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl<T: Add<Output = T>> Add for Vec4<T> {
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

impl<T: Sub<Output = T>> Sub for Vec2<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Vec3<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Vec4<T> {
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

impl<T: Copy + Mul<Output = T>> Mul<T> for Vec2<T> {
    type Output = Self;

    fn mul(self, other: T) -> Self::Output {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl<T : Copy + Mul<Output = T>> Mul<T> for Vec3<T> {
    type Output = Self;

    fn mul(self, other: T) -> Self::Output {
        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
        }
    }
}

impl<T : Copy + Mul<Output = T>> Mul<T> for Vec4<T> {
    type Output = Self;

    fn mul(self, other: T) -> Self::Output {
        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
            w: self.w * other,
        }
    }
}

#[derive(Debug, Default)]
pub struct Vec2Buffer<T : Copy> {
    backing : Vec<T>
}

impl<T : Copy> Deref for Vec2Buffer<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}


impl<T : Copy> Vec2Buffer<T> {
    pub fn new() -> Self {
        Self {
            backing : Vec::new()
        }
    }

    pub fn push(&mut self, x : T, y : T){
        self.backing.push(x);
        self.backing.push(y);
    }

    pub fn push_vec(&mut self, v : Vec2<T>){
        self.backing.push(v.x);
        self.backing.push(v.y);
    }

    pub fn len(&self) -> usize {
        self.backing.len()/2
    }

    pub fn set(&mut self, idx : usize, x : T, y : T) {
        self.backing[2*idx] = x;
        self.backing[2*idx + 1] = y;
    }

    pub fn set_vec(&mut self, idx : usize, v : Vec2<T>) {
        self.backing[2*idx] = v.x;
        self.backing[2*idx + 1] = v.y;
    }

    pub fn get(&self, idx : usize) -> Vec2<T> {
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
pub struct Vec3Buffer<T : Copy> {
    backing : Vec<T>
}

impl<T : Copy> Deref for Vec3Buffer<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}

impl<T : Copy> Vec3Buffer<T> {
    pub fn new() -> Self {
        Self {
            backing : Vec::new()
        }
    }

    pub fn push(&mut self, x : T, y : T, z : T){
        self.backing.push(x);
        self.backing.push(y);
        self.backing.push(z);
    }

    pub fn push_vec(&mut self, v : Vec3<T>){
        self.backing.push(v.x);
        self.backing.push(v.y);
        self.backing.push(v.z);
    }

    pub fn get(&self, idx : usize) -> Vec3<T> {
        Vec3 {
            x : self.backing[3*idx],
            y : self.backing[3*idx + 1],
            z : self.backing[3*idx + 2],
        }
    }

    pub fn set(&mut self, idx : usize, x : T, y : T, z : T) {
        self.backing[3*idx] = x;
        self.backing[3*idx + 1] = y;
        self.backing[3*idx + 2] = z;
    }

    pub fn set_vec(&mut self, idx : usize, v : Vec3<T>) {
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
pub struct Vec4Buffer<T : Copy> {
    backing : Vec<T>
}

impl<T : Copy> Deref for Vec4Buffer<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}

impl<T : Copy> Vec4Buffer<T> {
    pub fn new() -> Self {
        Self {
            backing : Vec::new()
        }
    }
    pub fn push(&mut self, x : T, y : T, z : T, w : T){
        self.backing.push(x);
        self.backing.push(y);
        self.backing.push(z);
        self.backing.push(w);
    }

    pub fn push_vec(&mut self, v : Vec4<T>){
        self.backing.push(v.x);
        self.backing.push(v.y);
        self.backing.push(v.z);
        self.backing.push(v.w);
    }

    pub fn get(&self, idx : usize) -> Vec4<T> {
        Vec4 {
            x : self.backing[4*idx],
            y : self.backing[4*idx + 1],
            z : self.backing[4*idx + 2],
            w : self.backing[4*idx + 3],
        }
    }

    pub fn set(&mut self, idx : usize, x : T, y : T, z : T, w : T) {
        self.backing[4*idx] = x;
        self.backing[4*idx + 1] = y;
        self.backing[4*idx + 2] = z;
        self.backing[4*idx + 3] = w;
    }

    pub fn set_vec(&mut self, idx : usize, v : Vec4<T>) {
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

impl Vec2<f32> {
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


impl Vec3<f32> {
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

impl Vec4<f32> {
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