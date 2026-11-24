//! DXF (Drawing Exchange Format) reading and writing.
//!
//! Reads the ASCII DXF files CAD applications export for laser cutting:
//! lines, arcs, circles, ellipses, polylines with bulges, splines and
//! text, including the block insertions that Onshape, Fusion and similar
//! tools wrap their export in.
//!
//! The reader never fails on a single unreadable entity. Anything it
//! could not represent is reported in
//! [`ReadOutput::issues`](crate::geo::read_output::ReadOutput), so a file
//! that is partly unsupported still imports the parts that are.
//!
//! Results are in native drawing units with Y pointing up. Use
//! [`units_to_mm`](crate::geo::read_output::ReadOutput::units_to_mm) to
//! convert, which reads the `$INSUNITS` header value and falls back to
//! millimetres for unitless files.
//!
//! Drawing DXF is handled by [`write`](crate::formats::dxf::writer::write), which
//! emits R12-compatible ASCII from a
//! [`Geometry`](crate::geo::geometry::Geometry).

pub mod entity;
pub mod entity_parser;
pub mod geometry_builder;
pub mod group_code;
pub mod ocs;
pub mod reader;
pub mod shapes;
pub mod writer;
