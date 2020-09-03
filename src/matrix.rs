#![allow(unused_imports)]

use std::ops::{Add, Sub, Mul};
use crate::vector::{Vec2, Vec3, Vec4};
use crate::rect::Rect;



#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Transform {
    pub data : [f32; 9]
}

impl Transform {
    pub fn new() -> Self {
        let data = [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0
        ];
        Self {
            data
        }
    }
    
	fn set(&mut self, m00 : f32, m01 : f32, m02 : f32, m10 : f32, m11 : f32, m12 : f32) {
		self.data = [
            m00, m10, 0.0,
            m01, m11, 0.0,
            m02, m12, 1.0
        ];
	}    

	pub fn translate(&mut self, x : f32, y : f32) {
        let [
            m00, m10, _,
            m01, m11, _,
            mut m02, mut m12, _
        ] = self.data;
        m02 += x * m00 + y * m01;
        m12 += x * m10 + y * m11;
        self.data[6] = m02;
        self.data[7] = m12;
	}

	pub fn rotate(&mut self, radians : f32) {
        let (s, c) = radians.sin_cos();
        let [
             m00,  m10, _,
             m01,  m11, _,
            _m02, _m12, _
        ] = self.data;        
		let v00 = c * m00 + s * m01;
		let v01 = c * m01 - s * m00;
		let v10 = c * m10 + s * m11;
        let v11 = c * m11 - s * m10;
        self.data[0] = v00;
        self.data[1] = v01;
        self.data[3] = v10;
        self.data[4] = v11;
	}

	pub fn scale_one(&mut self, d : f32) {
		self.scale(d, d);
	}



	pub fn scale(&mut self, x : f32, y : f32) {
		self.data[0] *= x;
		self.data[3] *= x;
		self.data[1] *= y;
		self.data[4] *= y;
	}

	pub fn multiply(&mut self, t : &Transform) {
        let [
            m00, m10, _,
            m01, m11, _,
            m02, m12, _
        ] = self.data;
        let [
            t_m00, t_m10, _,
            t_m01, t_m11, _,
            t_m02, t_m12, _
        ] = t.data;        
		self.data = [
            m00 * t_m00 + m01 * t_m10,
			m10 * t_m00 + m11 * t_m10,
            0.0,

			m00 * t_m01 + m01 * t_m11,
			m10 * t_m01 + m11 * t_m11,
            0.0,
            
            m00 * t_m02 + m01 * t_m12 + m02,
            m10 * t_m02 + m11 * t_m12 + m12,
            1.0
        ];
	}

	pub fn invert(&mut self) -> Result<(), ()> {
        let [
            m00, m10, _,
            m01, m11, _,
            m02, m12, _
        ] = self.data;
        let determinant = 1.0 / (m00 * m11 - m01 * m10);
		if !determinant.is_finite() {
			return Err(());
		}
		self.data = [
            m11 * determinant,
			-m10 * determinant,
            0.0,
			-m01 * determinant,
			m00 * determinant,
            0.0,
			(m01 * m12 - m11 * m02) * determinant,
            (m10 * m02 - m00 * m12) * determinant,
            1.0,
        ];
		Ok(())
	}
}

impl Transform {
	pub fn translate_vec(&mut self, v : Vec2<f32>) {
        self.translate(v.x, v.y);
	}

	pub fn scale_vec(&mut self, v : Vec2<f32>) {
		self.scale(v.x, v.y);
	}

	pub fn transform_point(&self, point : Vec2<f32>) -> Vec2<f32> {
        let [
            m00, m10, _,
            m01, m11, _,
            m02, m12, _
        ] = self.data;
        let Vec2 {x, y} = point;
        let x_result = m00 * x + m01 * y + m02;
		let y_result = m10 * x + m11 * y + m12;
		Vec2::new(x_result, y_result)
	}

	pub fn transform_direction(&self, direction : Vec2<f32>) -> Vec2<f32> {
        let [
            m00, m10, _,
            m01, m11, _,
            _m02, _m12, _
        ] = self.data;
        let Vec2 {x, y} = direction;
		let x_result = m00 * x + m01 * y;
		let y_result = m10 * x + m11 * y;
        Vec2::new(x_result, y_result)
    }
    
    pub fn transform_rect(&self, r : Rect) -> Rect {
        let top_left = Vec2::new(r.left(), r.top());
        let bottom_right = Vec2::new(r.right(), r.bottom());
        let tl_trans = self.transform_point(top_left);
        let br_trans = self.transform_point(bottom_right);
        let Vec2 {x : width, y : height} = br_trans - tl_trans;
        Rect::new(tl_trans.x, tl_trans.y, width, height)
    }
}






#[derive(Copy, Clone)]
pub struct Matrix3 {
    pub data : [f32; 9]
}

impl Matrix3 {
    pub const fn new(data : [f32; 9]) -> Self {
        Self {
            data
        }
    }

    pub fn row(&self, row : usize) -> Vec3<f32> {
        Vec3 {
            x : self.data[3 * row],
            y : self.data[3 * row + 1],
            z : self.data[3 * row + 2],
        }
    }

    pub fn det(&self) -> f32 {
        let mat = &self.data;
        mat[0] * mat[4] * mat[8] + mat[1] * mat[5] * mat[6] + mat[2] * mat[3] * mat[7] 
        - mat[0] * mat[5] * mat[7] - mat[1] * mat[3] * mat[8] - mat[2] * mat[4] * mat[6]
    }
}

impl Mul for Matrix3 {
    type Output = Self;

    fn mul(self, other : Matrix3) -> Self::Output {
        let mat1 = &self.data;
        let mat2 = &other.data;
        Self::new([
            mat1[0] * mat2[0] + mat1[1] * mat2[3] + mat1[2] * mat2[6], 
            mat1[0] * mat2[1] + mat1[1] * mat2[4] + mat1[2] * mat2[7], 
            mat1[0] * mat2[2] + mat1[1] * mat2[5] + mat1[2] * mat2[8], 
            mat1[3] * mat2[0] + mat1[4] * mat2[3] + mat1[5] * mat2[6], 
            mat1[3] * mat2[1] + mat1[4] * mat2[4] + mat1[5] * mat2[7], 
            mat1[3] * mat2[2] + mat1[4] * mat2[5] + mat1[5] * mat2[8], 
            mat1[6] * mat2[0] + mat1[7] * mat2[3] + mat1[8] * mat2[6], 
            mat1[6] * mat2[1] + mat1[7] * mat2[4] + mat1[8] * mat2[7], 
            mat1[6] * mat2[2] + mat1[7] * mat2[5] + mat1[8] * mat2[8]
        ])
    }
}

impl Mul<Vec3<f32>> for Matrix3 {
    type Output = Vec3<f32>;
    fn mul(self, other : Vec3<f32>) -> Self::Output {
        let mat1 = &self.data;
        Vec3::new(
            mat1[0] * other.x + mat1[1] * other.y + mat1[2] * other.z,
            mat1[3] * other.x + mat1[4] * other.y + mat1[5] * other.z,
            mat1[9] * other.x + mat1[7] * other.y + mat1[8] * other.z
        )
    }
}


#[derive(Copy, Clone)]
pub struct Matrix4 {
    pub data : [f32; 16]
}

impl Matrix4 {
    pub const fn new(data : [f32; 16]) -> Self {
        Self {
            data
        }
    }

    pub fn row(&self, row : usize) -> Vec4<f32> {
        Vec4 {
            x : self.data[4 * row],
            y : self.data[4 * row + 1],
            z : self.data[4 * row + 2],
            w : self.data[4 * row + 3],
        }
    }

    pub fn det(&self) -> f32 {
        let mat = &self.data;
        mat[0] * mat[5] * mat[10] * mat[15] + mat[0] * mat[6] * mat[11] * mat[13] 
        + mat[0] * mat[7] * mat[9] * mat[14] + mat[1] * mat[4] * mat[11] * mat[14] 
        + mat[1] * mat[6] * mat[8] * mat[15] + mat[1] * mat[7] * mat[10] * mat[12] 
        + mat[2] * mat[4] * mat[9] * mat[15] + mat[2] * mat[5] * mat[11] * mat[12] 
        + mat[2] * mat[7] * mat[8] * mat[13] + mat[3] * mat[4] * mat[10] * mat[13] 
        + mat[3] * mat[5] * mat[8] * mat[14] + mat[3] * mat[6] * mat[9] * mat[12] 
        - mat[0] * mat[5] * mat[11] * mat[14] - mat[0] * mat[6] * mat[9] * mat[15] 
        - mat[0] * mat[7] * mat[10] * mat[13] - mat[1] * mat[4] * mat[10] * mat[15] 
        - mat[1] * mat[6] * mat[11] * mat[12] - mat[1] * mat[7] * mat[8] * mat[14] 
        - mat[2] * mat[4] * mat[11] * mat[13] - mat[2] * mat[5] * mat[8] * mat[15] 
        - mat[2] * mat[7] * mat[9] * mat[12] - mat[3] * mat[4] * mat[9] * mat[14] 
        - mat[3] * mat[5] * mat[10] * mat[12] - mat[3] * mat[6] * mat[8] * mat[13]
    }
}

impl Mul for Matrix4 {
    type Output = Self;

    fn mul(self, other : Matrix4) -> Self::Output {
        let mat1 = self.data;
        let mat2 = other.data;
        Self::new([
            mat1[0] * mat2[0] + mat1[1] * mat2[4] + mat1[2] * mat2[8] + mat1[3] * mat2[12], 
            mat1[0] * mat2[1] + mat1[1] * mat2[5] + mat1[2] * mat2[9] + mat1[3] * mat2[13], 
            mat1[0] * mat2[2] + mat1[1] * mat2[6] + mat1[2] * mat2[10] + mat1[3] * mat2[14], 
            mat1[0] * mat2[3] + mat1[1] * mat2[7] + mat1[2] * mat2[11] + mat1[3] * mat2[15], 
            mat1[4] * mat2[0] + mat1[5] * mat2[4] + mat1[6] * mat2[8] + mat1[7] * mat2[12], 
            mat1[4] * mat2[1] + mat1[5] * mat2[5] + mat1[6] * mat2[9] + mat1[7] * mat2[13], 
            mat1[4] * mat2[2] + mat1[5] * mat2[6] + mat1[6] * mat2[10] + mat1[7] * mat2[14], 
            mat1[4] * mat2[3] + mat1[5] * mat2[7] + mat1[6] * mat2[11] + mat1[7] * mat2[15], 
            mat1[8] * mat2[0] + mat1[9] * mat2[4] + mat1[10] * mat2[8] + mat1[11] * mat2[12], 
            mat1[8] * mat2[1] + mat1[9] * mat2[5] + mat1[10] * mat2[9] + mat1[11] * mat2[13], 
            mat1[8] * mat2[2] + mat1[9] * mat2[6] + mat1[10] * mat2[10] + mat1[11] * mat2[14], 
            mat1[8] * mat2[3] + mat1[9] * mat2[7] + mat1[10] * mat2[11] + mat1[11] * mat2[15], 
            mat1[12] * mat2[0] + mat1[13] * mat2[4] + mat1[14] * mat2[8] + mat1[15] * mat2[12], 
            mat1[12] * mat2[1] + mat1[13] * mat2[5] + mat1[14] * mat2[9] + mat1[15] * mat2[13], 
            mat1[12] * mat2[2] + mat1[13] * mat2[6] + mat1[14] * mat2[10] + mat1[15] * mat2[14], 
            mat1[12] * mat2[3] + mat1[13] * mat2[7] + mat1[14] * mat2[11] + mat1[15] * mat2[15]
        ])
    }
}

impl Mul<Vec4<f32>> for Matrix4 {
    type Output = Vec4<f32>;
    fn mul(self, other : Vec4<f32>) -> Self::Output {
        let mat1 = &self.data;
        Vec4::new(
            mat1[0] * other.x + mat1[1] * other.y + mat1[2] * other.z + mat1[3] * other.w,
            mat1[4] * other.x + mat1[5] * other.y + mat1[6] * other.z + mat1[7] * other.w,
            mat1[8] * other.x + mat1[9] * other.y + mat1[10] * other.z + mat1[11] * other.w,
            mat1[12] * other.x + mat1[13] * other.y + mat1[14] * other.z + mat1[15] * other.w,
        )
    }
}