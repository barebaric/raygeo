use crate::geo::matrix::Matrix;
use crate::svg::parse_coords;

/// Parse an SVG `transform` attribute into a 3×3 affine matrix.
///
/// Supports: `translate`, `scale`, `rotate`, `skewX`, `skewY`, `matrix`.
/// Multiple functions can be chained (e.g. `translate(10,20) scale(2)`).
pub fn parse_svg_transform(transform_str: &str) -> Matrix {
    let mut matrix = Matrix::identity();
    if transform_str.is_empty() {
        return matrix;
    }
    let mut remaining = transform_str.trim();
    while !remaining.is_empty() {
        remaining = remaining
            .find(|c: char| c.is_ascii_alphabetic())
            .map_or("", |i| &remaining[i..]);
        let name_end = remaining
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(remaining.len());
        let name = &remaining[..name_end];
        remaining = remaining[name_end..].trim_start();
        if !remaining.starts_with('(') {
            break;
        }
        let close = match remaining.find(')') {
            Some(close) => close,
            None => break,
        };
        let coords = parse_coords(&remaining[1..close]);
        let fm = match name {
            "translate" => Matrix::from_translation(
                coords.first().copied().unwrap_or(0.0),
                coords.get(1).copied().unwrap_or(0.0),
            ),
            "scale" => {
                let sx = coords.first().copied().unwrap_or(1.0);
                Matrix::from_scale(sx, coords.get(1).copied().unwrap_or(sx))
            }
            "rotate" => match coords.first() {
                Some(&angle) if coords.len() >= 3 => Matrix::identity()
                    .rotate_pre(angle, Some((coords[1], coords[2]))),
                Some(&angle) => Matrix::from_rotation(angle),
                None => Matrix::identity(),
            },
            "skewX" => Matrix::from_shear(
                coords.first().map_or(0.0, |a| a.to_radians().tan()),
                0.0,
            ),
            "skewY" => Matrix::from_shear(
                0.0,
                coords.first().map_or(0.0, |a| a.to_radians().tan()),
            ),
            "matrix" => match TryInto::<[f64; 6]>::try_into(coords) {
                Ok([a, b, c, d, e, f]) => {
                    Matrix::from_cols_array(&[a, c, e, b, d, f, 0.0, 0.0, 1.0])
                }
                Err(_) => Matrix::identity(),
            },
            _ => Matrix::identity(),
        };
        matrix = matrix * fm;
        remaining = remaining[close + 1..].trim_start();
    }
    matrix
}
