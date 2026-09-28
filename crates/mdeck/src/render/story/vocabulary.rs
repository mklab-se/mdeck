//! The cast vocabulary: which kinds are people, and the words the AI prompt
//! may use for kinds, cells, fills and flow colours.

use super::Cell;

/// A cast member's look is a point cloud illustration named by its `kind`,
/// resolved through the illustration library (deck, user, built-in). These
/// are people: sized as figures and labelled in the brighter face.
pub const FIGURES: [&str; 7] = [
    "person",
    "hooded",
    "man",
    "woman",
    "thermographer",
    "presenter-up",
    "presenter-down",
];

pub fn is_figure(kind: &str) -> bool {
    FIGURES.contains(&kind)
}
/// Human-readable vocabulary for the AI prompt: the kinds a deck can cast
/// (its resolved illustration library) and the fixed cells, fills and colours.
pub fn vocabulary(kinds: &[String]) -> String {
    let kinds: Vec<&str> = kinds.iter().map(String::as_str).collect();
    let cells: Vec<&str> = Cell::ALL.iter().map(|c| c.name()).collect();
    format!(
        "kinds: {}\ncells: {}\nfills: outline, brain, hot, cold\nflow colors: white, ember, candle, pale",
        kinds.join(", "),
        cells.join(", ")
    )
}
