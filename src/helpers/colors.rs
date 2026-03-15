/// Predefined palette for account colors. Accounts are assigned colors
/// sequentially (by creation order) so the mapping is stable across imports.
pub const ACCOUNT_COLORS: &[&str] = &[
    "#3b82f6", // blue
    "#22c55e", // green
    "#a855f7", // purple
    "#ef4444", // red
    "#f59e0b", // amber
    "#06b6d4", // cyan
    "#ec4899", // pink
    "#84cc16", // lime
    "#6366f1", // indigo
    "#14b8a6", // teal
    "#f97316", // orange
    "#8b5cf6", // violet
];

/// Pick a color from the palette by index (wraps around).
pub fn color_by_index(index: usize) -> &'static str {
    ACCOUNT_COLORS[index % ACCOUNT_COLORS.len()]
}
