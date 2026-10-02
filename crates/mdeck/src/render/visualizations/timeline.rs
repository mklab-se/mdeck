use eframe::egui::{FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_FONT_PRIMARY_LABEL, VIZ_FONT_SECONDARY_LABEL, VIZ_OPACITY_AXIS, VIZ_OPACITY_LABEL,
    VIZ_STROKE_CONNECTOR, VIZ_STROKE_SEPARATOR, VIZ_TIMELINE_DOT, VizReveal, assign_steps,
    grammar::{Problem, Source},
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct TimelineEntry {
    date: String,
    description: String,
    reveal: VizReveal,
}

/// `- Date: Description`, or `- Description` without a date.
fn read(src: &Source) -> Vec<TimelineEntry> {
    src.check_settings(&[]);
    src.items
        .iter()
        .filter(|item| !item.text.is_empty())
        .map(|item| {
            item.check_attrs(src, &[]);
            let (date, description) = item.label_value().unwrap_or(("", item.text));
            TimelineEntry {
                date: date.to_string(),
                description: description.to_string(),
                reveal: item.reveal,
            }
        })
        .collect()
}

fn parse_timeline(content: &str) -> Vec<TimelineEntry> {
    read(&Source::parse(content))
}

/// The problems in a `@timeline` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}
// ─── Renderer ───────────────────────────────────────────────────────────────

/// An event's date and description centred on `anchor.x`: stacked up
/// from `anchor.y` (description on top) when `above`, else down from it
/// (date on top).
fn draw_event_text(
    cx: &super::VizCtx,
    entry: &TimelineEntry,
    (date_font, desc_font): (&FontId, &FontId),
    anchor: Pos2,
    max_label_width: f32,
    above: bool,
) {
    let super::VizCtx {
        theme,
        opacity,
        scale,
        ..
    } = *cx;
    let painter = cx.ui.painter();
    let (x, text_anchor_y) = (anchor.x, anchor.y);

    let date_color = Theme::with_opacity(theme.heading_color, opacity);
    let date_galley = painter.layout(
        entry.date.clone(),
        date_font.clone(),
        date_color,
        max_label_width,
    );
    let date_w = date_galley.rect.width();
    let date_h = date_galley.rect.height();

    let desc_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_LABEL);
    let desc_galley = painter.layout(
        entry.description.clone(),
        desc_font.clone(),
        desc_color,
        max_label_width,
    );
    let desc_w = desc_galley.rect.width();
    let desc_h = desc_galley.rect.height();

    if above {
        let desc_y = text_anchor_y - desc_h - date_h - 4.0 * scale;
        let date_y = text_anchor_y - date_h;
        painter.galley(Pos2::new(x - desc_w / 2.0, desc_y), desc_galley, desc_color);
        painter.galley(Pos2::new(x - date_w / 2.0, date_y), date_galley, date_color);
    } else {
        let date_y = text_anchor_y + 4.0 * scale;
        let desc_y = date_y + date_h + 2.0 * scale;
        painter.galley(Pos2::new(x - date_w / 2.0, date_y), date_galley, date_color);
        painter.galley(Pos2::new(x - desc_w / 2.0, desc_y), desc_galley, desc_color);
    }
}

/// Where the timeline's parts go: `n` events spaced along a horizontal line
/// through the middle of the chart.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TimelineLayout {
    line_y: f32,
    line_start: Pos2,
    line_end: Pos2,
    /// Left of the first event.
    first_x: f32,
    /// Where a lone event sits.
    center_x: f32,
    /// Distance between neighbouring events (0 for one event).
    spacing: f32,
    n: usize,
    dot_radius: f32,
    connector_len: f32,
}

fn timeline_layout(pos: Pos2, max_width: f32, height: f32, scale: f32, n: usize) -> TimelineLayout {
    let line_y = pos.y + height * 0.5;
    let margin = 80.0 * scale;
    let usable_width = max_width - margin * 2.0;
    let spacing = if n > 1 {
        usable_width / (n - 1) as f32
    } else {
        0.0
    };
    TimelineLayout {
        line_y,
        line_start: Pos2::new(pos.x + margin * 0.5, line_y),
        line_end: Pos2::new(pos.x + max_width - margin * 0.5, line_y),
        first_x: pos.x + margin,
        center_x: pos.x + max_width / 2.0,
        spacing,
        n,
        dot_radius: VIZ_TIMELINE_DOT * scale,
        connector_len: 50.0 * scale,
    }
}

impl TimelineLayout {
    /// Where event `i` sits along the line.
    fn event_x(&self, i: usize) -> f32 {
        if self.n > 1 {
            self.first_x + i as f32 * self.spacing
        } else {
            self.center_x
        }
    }

    /// The connector from the dot at `x` up (when `above`) or down to where
    /// the event's text is anchored: (start, end), the end being the anchor.
    fn connector(&self, x: f32, above: bool) -> (Pos2, Pos2) {
        let line_y = self.line_y;
        let (dot_radius, connector_len) = (self.dot_radius, self.connector_len);
        if above {
            (
                Pos2::new(x, line_y - dot_radius),
                Pos2::new(x, line_y - dot_radius - connector_len),
            )
        } else {
            (
                Pos2::new(x, line_y + dot_radius),
                Pos2::new(x, line_y + dot_radius + connector_len),
            )
        }
    }
}

pub fn draw_timeline(
    cx: &super::VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let super::VizCtx {
        ui,
        theme,
        opacity,
        scale,
        reveal_step,
        reveal_timestamp: _,
    } = *cx;
    let entries = parse_timeline(content);
    if entries.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    let reveals: Vec<VizReveal> = entries.iter().map(|e| e.reveal).collect();
    let steps = assign_steps(&reveals);
    let painter = ui.painter();

    let layout = timeline_layout(pos, max_width, height, scale, entries.len());

    // Draw the main timeline line
    let line_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_AXIS);
    painter.line_segment(
        [layout.line_start, layout.line_end],
        Stroke::new(VIZ_STROKE_SEPARATOR * scale, line_color),
    );
    crate::render::hints::push(
        ui.ctx(),
        crate::render::hints::Hint::Path(vec![layout.line_start, layout.line_end]),
    );

    let date_font = FontId::new(
        theme.body_size * VIZ_FONT_PRIMARY_LABEL * scale,
        theme.body_family(),
    );
    let desc_font = FontId::new(
        theme.body_size * VIZ_FONT_SECONDARY_LABEL * scale,
        theme.body_family(),
    );

    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }
        let anchor = draw_event_marker(cx, &layout, i);
        let fonts = (&date_font, &desc_font);
        let max_label_width = layout.spacing.max(120.0 * scale);
        let above = i.is_multiple_of(2);
        draw_event_text(cx, entry, fonts, anchor, max_label_width, above);
    }

    height
}

/// Event `i`'s dot on the line and its connector, alternating above and
/// below; returns where the connector ends, the anchor for its text.
fn draw_event_marker(cx: &super::VizCtx, layout: &TimelineLayout, i: usize) -> Pos2 {
    let super::VizCtx {
        theme,
        opacity,
        scale,
        ..
    } = *cx;
    let painter = cx.ui.painter();
    let palette = theme.edge_palette();
    let x = layout.event_x(i);
    let dot = Pos2::new(x, layout.line_y);
    let dot_radius = layout.dot_radius;

    let color = Theme::with_opacity(palette[i % palette.len()], opacity);

    // Dot on the line
    painter.circle_filled(dot, dot_radius, color);
    // White inner dot
    let inner_color = Theme::with_opacity(theme.background, opacity);
    painter.circle_filled(dot, dot_radius * 0.4, inner_color);

    // Connector line
    let (connector_start, connector_end) = layout.connector(x, i.is_multiple_of(2));
    painter.line_segment(
        [connector_start, connector_end],
        Stroke::new(
            VIZ_STROKE_CONNECTOR * scale,
            Theme::with_opacity(color, opacity * 0.6),
        ),
    );
    connector_end
}
// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timeline_basic() {
        let content = "- 2000: Y2K Bug\n+ 2007: iPhone Released";
        let entries = parse_timeline(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].date, "2000");
        assert_eq!(entries[0].description, "Y2K Bug");
        assert_eq!(entries[0].reveal, VizReveal::Static);
        assert_eq!(entries[1].date, "2007");
        assert_eq!(entries[1].description, "iPhone Released");
        assert_eq!(entries[1].reveal, VizReveal::NextStep);
    }

    #[test]
    fn test_parse_timeline_no_description() {
        let content = "- Just a label";
        let entries = parse_timeline(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].date, "");
        assert_eq!(entries[0].description, "Just a label");
    }

    #[test]
    fn test_parse_timeline_skips_comments() {
        let content = "# header\n- 2020: Event\n# note";
        let entries = parse_timeline(content);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_parse_timeline_reveal_markers() {
        let content = "- A: first\n+ B: second\n* C: third";
        let entries = parse_timeline(content);
        assert_eq!(entries[0].reveal, VizReveal::Static);
        assert_eq!(entries[1].reveal, VizReveal::NextStep);
        assert_eq!(entries[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_timeline_layout_spaces_events_along_the_line() {
        let layout = timeline_layout(Pos2::new(10.0, 20.0), 1000.0, 400.0, 1.0, 3);
        assert_eq!(layout.line_y, 220.0);
        assert_eq!(layout.line_start, Pos2::new(50.0, 220.0));
        assert_eq!(layout.line_end, Pos2::new(970.0, 220.0));
        assert_eq!(layout.spacing, 420.0);
        assert_eq!(layout.event_x(0), 90.0);
        assert_eq!(layout.event_x(2), 930.0);
    }

    #[test]
    fn test_timeline_layout_centres_a_lone_event() {
        let layout = timeline_layout(Pos2::new(10.0, 20.0), 1000.0, 400.0, 1.0, 1);
        assert_eq!(layout.spacing, 0.0);
        assert_eq!(layout.event_x(0), 510.0);
    }

    #[test]
    fn test_timeline_connector_alternates_sides() {
        let layout = timeline_layout(Pos2::new(0.0, 0.0), 1000.0, 400.0, 1.0, 2);
        let r = layout.dot_radius;
        let (start, end) = layout.connector(100.0, true);
        assert_eq!(start, Pos2::new(100.0, 200.0 - r));
        assert_eq!(end, Pos2::new(100.0, 200.0 - r - 50.0));
        let (start, end) = layout.connector(100.0, false);
        assert_eq!(start, Pos2::new(100.0, 200.0 + r));
        assert_eq!(end, Pos2::new(100.0, 200.0 + r + 50.0));
    }
}
