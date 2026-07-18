use crate::ops::container::Ops;
use crate::ops::enums::{CommandCategory, CommandType};

/// Apply bidirectional scan offset to right-to-left raster passes.
///
/// For every raster pass (a `MoveTo` followed by a `ScanLine`), if the
/// pass runs right-to-left (`scan_end.x < move_end.x`), both the entry
/// `MoveTo` and the `ScanLine` endpoint are shifted along X by
/// `offset_mm`.  Left-to-right passes are transferred unchanged.
pub fn apply_bidir_scan_offset(ops: &mut Ops, offset_mm: f64) {
    if ops.is_empty() || offset_mm == 0.0 {
        return;
    }

    let source = ops.copy();
    ops.clear();
    let n = source.len();
    let mut idx = 0;

    while idx < n {
        if source.command_type(idx) == CommandType::MoveTo {
            let move_end = source.endpoint(idx);

            // Skip STATE commands between MoveTo and potential ScanLine.
            let mut j = idx + 1;
            while j < n && source.category(j) == CommandCategory::State {
                j += 1;
            }

            if j < n && source.is_scanline(j) {
                let scan_end = source.endpoint(j);

                if scan_end.x < move_end.x {
                    // Right-to-left: shift both endpoints by offset_mm.
                    let extra = source.extra_axes(idx).map(|ea| {
                        ea.iter().map(|(a, v)| (*a, *v)).collect::<Vec<_>>()
                    });
                    ops.move_to(
                        move_end.x + offset_mm,
                        move_end.y,
                        move_end.z,
                        extra,
                    );
                    // Transfer state commands between MoveTo and ScanLine.
                    for k in (idx + 1)..j {
                        ops.transfer_command_from(&source, k);
                    }
                    let scan_extra = source.extra_axes(j).map(|ea| {
                        ea.iter().map(|(a, v)| (*a, *v)).collect::<Vec<_>>()
                    });
                    ops.scan_to(
                        scan_end.x + offset_mm,
                        scan_end.y,
                        scan_end.z,
                        source.scanline_data(j),
                        scan_extra,
                    );
                } else {
                    // Left-to-right or zero-length: transfer unchanged.
                    for k in idx..=j {
                        ops.transfer_command_from(&source, k);
                    }
                }
                idx = j + 1;
                continue;
            }
        }
        ops.transfer_command_from(&source, idx);
        idx += 1;
    }
}
