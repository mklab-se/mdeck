//! `MDECK_EXPORT_AT` / `MDECK_EXPORT_MOMENT`: stills of an engine's motion.

/// Developer settings for looking at an engine's motion in export.
pub(super) struct Rehearsal {
    pub(super) at: Option<f32>,
    pub(super) countdown: Option<(crate::engines::CountPhase, f32)>,
    pub(super) end: bool,
}

impl Rehearsal {
    pub(super) fn from_env() -> Self {
        let at = std::env::var("MDECK_EXPORT_AT")
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok());
        let moment = std::env::var("MDECK_EXPORT_MOMENT").unwrap_or_default();
        let countdown = match moment.trim() {
            "3" => Some((crate::engines::CountPhase::Digit(3), 0.5)),
            "2" => Some((crate::engines::CountPhase::Digit(2), 0.5)),
            "1" => Some((crate::engines::CountPhase::Digit(1), 0.5)),
            "burst" => Some((crate::engines::CountPhase::Burst, 0.0)),
            _ => None,
        };
        Rehearsal {
            at,
            countdown,
            end: moment.trim() == "end",
        }
    }
}
