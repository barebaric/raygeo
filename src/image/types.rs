/// A raw pixel image buffer stored on a higher-layer part entity.
///
/// The buffer is a flat row-major uint8 array of shape `(height, width)`.
/// Each assembler interprets the pixel values as needed (grayscale for
/// raster, binary for shrinkwrap, etc.).
#[derive(Clone, Debug)]
pub struct PixelImage {
    pub data: Vec<u8>,
    pub height: usize,
    pub width: usize,
}
