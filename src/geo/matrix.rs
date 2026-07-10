use glam::{DMat3, DMat4, DVec2, DVec3, DVec4};

#[derive(Clone, Debug)]
pub struct Matrix {
    pub m: DMat3,
}

/// Computes Frobenius norm squared for a DMat3
fn mat3_norm_sq(m: &DMat3) -> f64 {
    m.x_axis.length_squared()
        + m.y_axis.length_squared()
        + m.z_axis.length_squared()
}

impl Matrix {
    pub fn identity() -> Self {
        Matrix { m: DMat3::IDENTITY }
    }

    pub fn from_cols_array(data: &[f64; 9]) -> Self {
        Matrix {
            m: DMat3::from_cols(
                DVec3::new(data[0], data[3], 0.0),
                DVec3::new(data[1], data[4], 0.0),
                DVec3::new(data[2], data[5], 1.0),
            ),
        }
    }

    pub fn from_cols_arrays(data: [[f64; 3]; 3]) -> Self {
        Matrix::from_cols_array(&[
            data[0][0], data[0][1], data[0][2], data[1][0], data[1][1],
            data[1][2], data[2][0], data[2][1], data[2][2],
        ])
    }

    pub fn from_4x4(m44: DMat4) -> Self {
        Matrix {
            m: DMat3::from_cols(
                DVec3::new(m44.x_axis.x, m44.y_axis.x, 0.0),
                DVec3::new(m44.x_axis.y, m44.y_axis.y, 0.0),
                DVec3::new(m44.x_axis.z, m44.y_axis.z, 1.0),
            ),
        }
    }

    pub fn from_translation(tx: f64, ty: f64) -> Self {
        Matrix {
            m: DMat3::from_cols(DVec3::X, DVec3::Y, DVec3::new(tx, ty, 1.0)),
        }
    }

    pub fn from_scale(sx: f64, sy: f64) -> Self {
        Matrix {
            m: DMat3::from_cols(
                DVec3::new(sx, 0.0, 0.0),
                DVec3::new(0.0, sy, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
            ),
        }
    }

    pub fn from_rotation(angle_deg: f64) -> Self {
        let rad = angle_deg.to_radians();
        Matrix {
            m: DMat3::from_rotation_z(rad),
        }
    }

    pub fn from_shear(shx: f64, shy: f64) -> Self {
        Matrix {
            m: DMat3::from_cols(
                DVec3::new(1.0, shy, 0.0),
                DVec3::new(shx, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
            ),
        }
    }

    pub fn from_compose(
        tx: f64,
        ty: f64,
        angle_deg: f64,
        sx: f64,
        sy: f64,
        skew_angle_deg: f64,
    ) -> Self {
        let r = Self::from_rotation(angle_deg);
        let s = Self::from_scale(sx, sy);
        let skew_rad = skew_angle_deg.to_radians();
        let shear_factor = skew_rad.tan();
        let k = Self::from_shear(shear_factor, 0.0);
        let mut linear = r * s * k;
        linear.set_translation_in_place(tx, ty);
        linear
    }

    pub fn copy(&self) -> Self {
        self.clone()
    }

    /// Index into row-major layout: m[row][col]
    /// Column 0 = x_axis, Column 1 = y_axis, Column 2 = z_axis
    pub fn get(&self, row: usize, col: usize) -> f64 {
        match col {
            0 => self.m.x_axis[row],
            1 => self.m.y_axis[row],
            2 => self.m.z_axis[row],
            _ => panic!("column index out of bounds"),
        }
    }

    fn get_linear_part(&self) -> (f64, f64, f64, f64) {
        (
            self.get(0, 0),
            self.get(1, 0),
            self.get(0, 1),
            self.get(1, 1),
        )
    }

    pub fn determinant_2x2(&self) -> f64 {
        let (a, b, c, d) = self.get_linear_part();
        a * d - b * c
    }

    pub fn is_identity(&self) -> bool {
        mat3_norm_sq(&(self.m - DMat3::IDENTITY)) < 1e-12
    }

    pub fn is_flipped(&self) -> bool {
        self.determinant_2x2() < 0.0
    }

    pub fn has_zero_scale(&self, tolerance: f64) -> bool {
        let (sx, sy) = self.abs_scale();
        sx < tolerance || sy < tolerance
    }

    pub fn is_close(&self, other: &Matrix, tol: f64) -> bool {
        mat3_norm_sq(&(self.m - other.m)) < tol * tol
    }

    pub fn translation(&self) -> (f64, f64) {
        (self.get(0, 2), self.get(1, 2))
    }

    pub fn set_translation(&self, tx: f64, ty: f64) -> Self {
        let mut m = self.clone();
        m.set_translation_in_place(tx, ty);
        m
    }

    fn set_translation_in_place(&mut self, tx: f64, ty: f64) {
        self.m.z_axis.x = tx;
        self.m.z_axis.y = ty;
    }

    pub fn without_translation(&self) -> Self {
        let mut m = self.clone();
        m.m.z_axis.x = 0.0;
        m.m.z_axis.y = 0.0;
        m
    }

    pub fn scale(&self) -> (f64, f64) {
        let (_, _, _, sx, sy, _) = self.decompose();
        (sx, sy)
    }

    pub fn abs_scale(&self) -> (f64, f64) {
        let (sx, sy) = self.scale();
        (sx.abs(), sy.abs())
    }

    pub fn rotation(&self) -> f64 {
        let (_, _, angle_deg, _, _, _) = self.decompose();
        angle_deg
    }

    pub fn x_axis_angle(&self) -> f64 {
        let xx = self.get(0, 0);
        let yx = self.get(1, 0);
        yx.atan2(xx).to_degrees()
    }

    pub fn y_axis_angle(&self) -> f64 {
        let xy = self.get(0, 1);
        let yy = self.get(1, 1);
        yy.atan2(xy).to_degrees()
    }

    pub fn translate_pre(&self, tx: f64, ty: f64) -> Self {
        Self::from_translation(tx, ty) * self.clone()
    }

    pub fn translate_post(&self, tx: f64, ty: f64) -> Self {
        self.clone() * Self::from_translation(tx, ty)
    }

    pub fn scale_pre(
        &self,
        sx: f64,
        sy: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let s = wrap_center(Self::from_scale(sx, sy), center);
        s * self.clone()
    }

    pub fn scale_post(
        &self,
        sx: f64,
        sy: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let s = wrap_center(Self::from_scale(sx, sy), center);
        self.clone() * s
    }

    pub fn rotate_pre(
        &self,
        angle_deg: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let r = wrap_center(Self::from_rotation(angle_deg), center);
        r * self.clone()
    }

    pub fn rotate_post(
        &self,
        angle_deg: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let r = wrap_center(Self::from_rotation(angle_deg), center);
        self.clone() * r
    }

    pub fn shear_pre(
        &self,
        shx: f64,
        shy: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let k = wrap_center(Self::from_shear(shx, shy), center);
        k * self.clone()
    }

    pub fn shear_post(
        &self,
        shx: f64,
        shy: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let k = wrap_center(Self::from_shear(shx, shy), center);
        self.clone() * k
    }

    pub fn flip_horizontal(center: Option<(f64, f64)>) -> Self {
        wrap_center(Self::from_scale(-1.0, 1.0), center)
    }

    pub fn flip_vertical(center: Option<(f64, f64)>) -> Self {
        wrap_center(Self::from_scale(1.0, -1.0), center)
    }

    pub fn invert(&self) -> Self {
        Matrix {
            m: self.m.inverse(),
        }
    }

    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        let r = self.m.transform_point2(DVec2::new(x, y));
        (r.x, r.y)
    }

    pub fn transform_vector(&self, vx: f64, vy: f64) -> (f64, f64) {
        let r = self.m.transform_vector2(DVec2::new(vx, vy));
        (r.x, r.y)
    }

    pub fn transform_rectangle(
        &self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> (f64, f64, f64, f64) {
        let c1 = self.transform_point(x, y);
        let c2 = self.transform_point(x + w, y);
        let c3 = self.transform_point(x + w, y + h);
        let c4 = self.transform_point(x, y + h);
        let min_x = c1.0.min(c2.0).min(c3.0).min(c4.0);
        let min_y = c1.1.min(c2.1).min(c3.1).min(c4.1);
        let max_x = c1.0.max(c2.0).max(c3.0).max(c4.0);
        let max_y = c1.1.max(c2.1).max(c3.1).max(c4.1);
        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    pub fn for_cairo(&self) -> (f64, f64, f64, f64, f64, f64) {
        (
            self.get(0, 0),
            self.get(1, 0),
            self.get(0, 1),
            self.get(1, 1),
            self.get(0, 2),
            self.get(1, 2),
        )
    }

    pub fn to_cols_array(&self) -> [f64; 9] {
        [
            self.get(0, 0),
            self.get(0, 1),
            self.get(0, 2),
            self.get(1, 0),
            self.get(1, 1),
            self.get(1, 2),
            0.0,
            0.0,
            1.0,
        ]
    }

    pub fn to_4x4(&self) -> DMat4 {
        DMat4::from_cols(
            DVec4::new(self.get(0, 0), self.get(1, 0), 0.0, 0.0),
            DVec4::new(self.get(0, 1), self.get(1, 1), 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0, 0.0),
            DVec4::new(self.get(0, 2), self.get(1, 2), 0.0, 1.0),
        )
    }

    pub fn decompose(&self) -> (f64, f64, f64, f64, f64, f64) {
        let tx = self.get(0, 2);
        let ty = self.get(1, 2);

        let (a, b, c, d) = self.get_linear_part();

        let sx = (a * a + b * b).sqrt();

        let angle_rad = b.atan2(a);
        let angle_deg = angle_rad.to_degrees();

        let det = a * d - b * c;
        let sy = if sx != 0.0 {
            det / sx
        } else {
            (c * c + d * d).sqrt()
        };

        let skew_angle_deg = if sx != 0.0 {
            let cos_r = angle_rad.cos();
            let sin_r = angle_rad.sin();
            let shear_factor = (cos_r * c + sin_r * d) / sx;
            shear_factor.atan().to_degrees()
        } else {
            0.0
        };

        (tx, ty, angle_deg, sx, sy, skew_angle_deg)
    }
}

fn wrap_center(m: Matrix, center: Option<(f64, f64)>) -> Matrix {
    match center {
        Some((cx, cy)) => {
            let t1 = Matrix::from_translation(-cx, -cy);
            let t2 = Matrix::from_translation(cx, cy);
            t2 * m * t1
        }
        None => m,
    }
}

impl std::ops::Mul for Matrix {
    type Output = Matrix;
    fn mul(self, rhs: Matrix) -> Matrix {
        Matrix { m: self.m * rhs.m }
    }
}

impl std::ops::Mul<&Matrix> for Matrix {
    type Output = Matrix;
    fn mul(self, rhs: &Matrix) -> Matrix {
        Matrix { m: self.m * rhs.m }
    }
}

impl std::ops::Mul<Matrix> for &Matrix {
    type Output = Matrix;
    fn mul(self, rhs: Matrix) -> Matrix {
        Matrix { m: self.m * rhs.m }
    }
}

impl std::ops::Mul for &Matrix {
    type Output = Matrix;
    fn mul(self, rhs: &Matrix) -> Matrix {
        Matrix { m: self.m * rhs.m }
    }
}

impl PartialEq for Matrix {
    fn eq(&self, other: &Self) -> bool {
        mat3_norm_sq(&(self.m - other.m)) < 1e-12
    }
}
