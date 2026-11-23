//! QR Code encoding via the `qrcodegen` crate.
//!
//! The segment builder picks the smallest mixed numeric, alphanumeric
//! and byte-mode segmentation for the text; the requested error level is
//! kept exactly instead of being boosted to a higher level.

use qrcodegen::{QrCode, QrCodeEcc, QrSegment, Version};

use super::{BarcodeError, Encoded, QrErrorCorrection, Symbology};

/// Encodes `text` as a QR Code.
///
/// The smallest QR version that fits the text is selected automatically.
/// The error-correction level is not boosted, so the returned symbol
/// always uses exactly the requested level.
pub fn encode(
    text: &str,
    error_correction: QrErrorCorrection,
) -> Result<Encoded, BarcodeError> {
    if text.is_empty() {
        return Err(BarcodeError::EmptyContent);
    }
    let segments = QrSegment::make_segments(text);
    let code = QrCode::encode_segments_advanced(
        &segments,
        ecc(error_correction),
        Version::MIN,
        Version::MAX,
        None,
        false,
    )
    .map_err(|_| BarcodeError::QrTooLong)?;

    let size = code.size() as usize;
    let modules = (0..size)
        .map(|y| {
            (0..size)
                .map(|x| code.get_module(x as i32, y as i32))
                .collect()
        })
        .collect();

    Ok(Encoded {
        symbology: Symbology::Qr,
        modules,
        text: String::new(),
        guards: Vec::new(),
    })
}

fn ecc(error_correction: QrErrorCorrection) -> QrCodeEcc {
    match error_correction {
        QrErrorCorrection::Low => QrCodeEcc::Low,
        QrErrorCorrection::Medium => QrCodeEcc::Medium,
        QrErrorCorrection::Quartile => QrCodeEcc::Quartile,
        QrErrorCorrection::High => QrCodeEcc::High,
    }
}
