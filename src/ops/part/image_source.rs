//! Lazy source of pixel data for assemblers that need a raster image.
//
//! [`Part`](super::Part) carries an [`ImageSource`] trait object so
//! assemblers can pull pixels lazily rather than reading a fully
//! materialised buffer up front.

use crate::image::types::PixelImage;

/// Lazy access to a raster image for assemblers.
///
/// Implementations must be `Send + Sync` because the part may be
/// read from rayon workers. Methods are object-safe; the source is
/// held as `Box<dyn ImageSource>` on [`Part`](super::Part).
pub trait ImageSource: Send + Sync {
    /// Pixel dimensions of the image as `(width, height)`.
    ///
    /// Must not allocate pixel data; assemblers call this for layout
    /// and bounding decisions before any read.
    fn dimensions(&self) -> (u32, u32);

    /// Pull a horizontal slab `[y_start, y_end)` into `dst`.
    ///
    /// `dst` must be large enough to receive `(y_end - y_start) *
    /// width` bytes. Returns the number of rows actually written —
    /// this is the requested height except for a clipped final slab,
    /// in which case it is `height - y_start`.
    ///
    /// Implementations that always have the full image in memory
    /// simply copy the requested rows; implementations that pull on
    /// demand perform the I/O here.
    fn read_slab(&self, y_start: u32, y_end: u32, dst: &mut [u8]) -> u32;

    /// Return the full image buffer as a flat row-major `Vec<u8>`.
    ///
    /// Returns `None` when the source cannot materialise the whole
    /// image (e.g. a chunked source larger than available memory).
    /// Assemblers that require the full buffer (e.g. shrinkwrap's
    /// concave-hull pass) must degrade or refuse when this returns
    /// `None`. Assemblers that can work in slabs should prefer
    /// [`read_slab`](Self::read_slab) and avoid this method.
    fn read_all(&self) -> Option<Vec<u8>>;

    /// Cancellation poll. Called frequently by long-running
    /// assemblers. Default impl always returns `false`.
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// An [`ImageSource`] backed by a single in-memory `PixelImage`.
///
/// The entire buffer is held in memory and slab reads are cheap
/// copies. Future implementations may pull slabs from a chunked
/// source on demand.
#[derive(Clone, Debug)]
pub struct WholeImageSource {
    pub data: Vec<u8>,
    pub height: u32,
    pub width: u32,
}

impl WholeImageSource {
    pub fn new(image: PixelImage) -> Self {
        let PixelImage {
            data,
            height,
            width,
        } = image;
        WholeImageSource {
            data,
            height: height as u32,
            width: width as u32,
        }
    }
}

impl ImageSource for WholeImageSource {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn read_slab(&self, y_start: u32, y_end: u32, dst: &mut [u8]) -> u32 {
        let y_start = y_start.min(self.height);
        let y_end_clamped = y_end.min(self.height);
        if y_end_clamped <= y_start {
            return 0;
        }
        let row_bytes = self.width as usize;
        let rows = (y_end_clamped - y_start) as usize;
        let needed = rows * row_bytes;
        assert!(
            dst.len() >= needed,
            "read_slab: dst too small (have {}, need {} bytes for {} rows of width {})",
            dst.len(),
            needed,
            rows,
            self.width,
        );
        let start = y_start as usize * row_bytes;
        let end = start + needed;
        dst[..needed].copy_from_slice(&self.data[start..end]);
        rows as u32
    }

    fn read_all(&self) -> Option<Vec<u8>> {
        Some(self.data.clone())
    }
}
