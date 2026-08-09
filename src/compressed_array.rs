//! Compressed in-memory array storage for large numpy buffers.
//!
//! Provides zstd-based compression of typed arrays so that scene
//! compilation output (vertex buffers, power textures) can be kept
//! compressed in memory and only decompressed on demand at GPU
//! upload time.

use std::io::Read;

const COMPRESS_THRESHOLD: usize = 4096;
const ZSTD_LEVEL: i32 = 3;

#[derive(Clone, Copy, PartialEq)]
pub enum Dtype {
    Float32,
    Int32,
    UInt8,
}

impl Dtype {
    pub fn item_size(&self) -> usize {
        match self {
            Dtype::Float32 | Dtype::Int32 => 4,
            Dtype::UInt8 => 1,
        }
    }
}

/// Compressed (or raw, if below threshold) byte buffer with shape and
/// dtype metadata.  Pure Rust — no PyO3 dependency.
pub struct CompressedArray {
    pub data: Vec<u8>,
    pub shape: Vec<usize>,
    pub dtype: Dtype,
    pub is_compressed: bool,
    pub uncompressed_size: usize,
}

impl CompressedArray {
    fn from_bytes(raw: &[u8], shape: Vec<usize>, dtype: Dtype) -> Self {
        let uncompressed_size = raw.len();
        if raw.len() < COMPRESS_THRESHOLD {
            return Self {
                data: raw.to_vec(),
                shape,
                dtype,
                is_compressed: false,
                uncompressed_size,
            };
        }
        let compressed =
            zstd::encode_all(raw, ZSTD_LEVEL).expect("zstd compression");
        Self {
            data: compressed,
            shape,
            dtype,
            is_compressed: true,
            uncompressed_size,
        }
    }

    pub fn from_vec_f32(data: Vec<f32>) -> Self {
        let shape = vec![data.len()];
        let bytes = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * 4,
            )
        };
        Self::from_bytes(bytes, shape, Dtype::Float32)
    }

    pub fn from_vec_i32(data: Vec<i32>) -> Self {
        let shape = vec![data.len()];
        let bytes = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * 4,
            )
        };
        Self::from_bytes(bytes, shape, Dtype::Int32)
    }

    pub fn from_vec_u8(data: Vec<u8>, shape: Vec<usize>) -> Self {
        Self::from_bytes(&data, shape, Dtype::UInt8)
    }

    /// Decompress into the provided buffer.
    pub fn decompress_into(&self, dst: &mut [u8]) -> Result<(), String> {
        if self.is_compressed {
            zstd::Decoder::new(self.data.as_slice())
                .map_err(|e| e.to_string())?
                .read_exact(dst)
                .map_err(|e| e.to_string())?;
        } else {
            dst.copy_from_slice(&self.data);
        }
        Ok(())
    }

    /// Decompress into a typed Vec, allocating with correct alignment.
    pub fn decompress_to_vec_f32(&self) -> Result<Vec<f32>, String> {
        let count = self.uncompressed_size / 4;
        let mut data: Vec<f32> = vec![0.0; count];
        let dst = unsafe {
            std::slice::from_raw_parts_mut(
                data.as_mut_ptr() as *mut u8,
                self.uncompressed_size,
            )
        };
        self.decompress_into(dst)?;
        Ok(data)
    }

    pub fn decompress_to_vec_i32(&self) -> Result<Vec<i32>, String> {
        let count = self.uncompressed_size / 4;
        let mut data: Vec<i32> = vec![0; count];
        let dst = unsafe {
            std::slice::from_raw_parts_mut(
                data.as_mut_ptr() as *mut u8,
                self.uncompressed_size,
            )
        };
        self.decompress_into(dst)?;
        Ok(data)
    }

    pub fn decompress_to_vec_u8(&self) -> Result<Vec<u8>, String> {
        let mut data: Vec<u8> = vec![0; self.uncompressed_size];
        self.decompress_into(&mut data)?;
        Ok(data)
    }
}
