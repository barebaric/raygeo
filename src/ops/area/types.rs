/// How the cleared area merges new swept polygons into stored fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateStrategy {
    /// Union ALL fragments with the new polygon(s) in one pass.
    #[default]
    Global,
    /// Only union fragments whose bbox overlaps the new polygon.
    Local,
}
