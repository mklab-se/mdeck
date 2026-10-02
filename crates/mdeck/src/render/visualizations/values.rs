//! Parsing values as written in presentation markdown.

/// Parse a single numeric value as written in presentation markdown.
///
/// Accepts plain numbers (`40`, `3.5`, `-2`, `1e3`), currency prefixes (`$40`,
/// `€40`, `£40`), a `%` suffix (`12%`), `_` digit separators (`1_000`),
/// thousands separators in groups of three (`1,000`, `12,345.5`), and a
/// trailing unit made of letters (`40 units`, `4.2M`; the unit is dropped,
/// not scaled). Returns `None` for anything else and for non-finite numbers
/// (`inf`, `nan`), which would otherwise make axis loops run forever.
pub fn parse_value(raw: &str) -> Option<f32> {
    let s = raw.trim();
    let s = s.trim_start_matches(['$', '€', '£']).trim_start();
    let s = s.trim_end_matches('%').trim_end();
    let cleaned: String = s.chars().filter(|&c| c != '_').collect();
    if cleaned.is_empty() {
        return None;
    }

    // Fast path: a plain number (also covers scientific notation).
    if let Ok(v) = cleaned.parse::<f32>() {
        return v.is_finite().then_some(v);
    }

    // Split into a numeric prefix and an optional alphabetic unit suffix.
    let split = cleaned
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+')))
        .map_or(cleaned.len(), |(i, _)| i);
    let (number, unit) = cleaned.split_at(split);
    if !unit.chars().all(|c| c.is_alphabetic() || c.is_whitespace()) {
        return None;
    }

    let number = if number.contains(',') {
        if !is_thousands_grouped(number) {
            return None;
        }
        number.replace(',', "")
    } else {
        number.to_string()
    };
    number.parse::<f32>().ok().filter(|v| v.is_finite())
}

/// True when `s` is a number whose commas are all thousands separators:
/// an optional sign, 1-3 digits, then groups of exactly three digits,
/// optionally followed by a decimal fraction.
fn is_thousands_grouped(s: &str) -> bool {
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s, None),
    };
    if let Some(f) = frac_part
        && (f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    let mut groups = int_part.split(',');
    let Some(first) = groups.next() else {
        return false;
    };
    if first.is_empty() || first.len() > 3 || !first.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let mut any = false;
    for g in groups {
        if g.len() != 3 || !g.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        any = true;
    }
    any
}

/// Remove thousands separators from a comma-separated list of values.
///
/// A comma directly followed by exactly three digits (`1,000`) is only treated
/// as a thousands separator when the list also uses `", "` (comma + space) to
/// separate its items, i.e. `1,000, 2,000`. A list written without spaces
/// (`100,200,300`) keeps every comma as an item separator.
pub fn strip_thousands_separators(list: &str) -> String {
    if !list.contains(", ") {
        return list.to_string();
    }
    let bytes = list.as_bytes();
    let mut out = String::with_capacity(list.len());
    for (i, &b) in bytes.iter().enumerate() {
        if b == b','
            && i > 0
            && bytes[i - 1].is_ascii_digit()
            && bytes.len() >= i + 4
            && bytes[i + 1..i + 4].iter().all(u8::is_ascii_digit)
            && !bytes.get(i + 4).is_some_and(u8::is_ascii_digit)
        {
            continue;
        }
        out.push(b as char);
    }
    out
}

/// Split a `"Label: value"` item into its label and numeric value.
/// Returns `None` when there is no `": "` or the value does not parse.
pub fn parse_label_value(text: &str) -> Option<(String, f32)> {
    let (label, value) = text.split_once(": ")?;
    let value = parse_value(value)?;
    Some((label.trim().to_string(), value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_value_plain_and_decorated() {
        assert_eq!(parse_value("40"), Some(40.0));
        assert_eq!(parse_value(" 3.5 "), Some(3.5));
        assert_eq!(parse_value("-2"), Some(-2.0));
        assert_eq!(parse_value("1e3"), Some(1000.0));
        assert_eq!(parse_value("12%"), Some(12.0));
        assert_eq!(parse_value("$40"), Some(40.0));
        assert_eq!(parse_value("€ 40"), Some(40.0));
        assert_eq!(parse_value("£1,250.5"), Some(1250.5));
        assert_eq!(parse_value("1_000"), Some(1000.0));
        assert_eq!(parse_value("40 units"), Some(40.0));
        assert_eq!(parse_value("4.2M"), Some(4.2));
        assert_eq!(parse_value("1,000"), Some(1000.0));
        assert_eq!(parse_value("1,000,000"), Some(1_000_000.0));
    }

    #[test]
    fn test_parse_value_rejects_garbage_and_non_finite() {
        assert_eq!(parse_value("inf"), None);
        assert_eq!(parse_value("-infinity"), None);
        assert_eq!(parse_value("nan"), None);
        assert_eq!(parse_value("NaN"), None);
        assert_eq!(parse_value("1e40"), None);
        assert_eq!(parse_value(""), None);
        assert_eq!(parse_value("abc"), None);
        assert_eq!(parse_value("1,00"), None);
        assert_eq!(parse_value("1,0000"), None);
        assert_eq!(parse_value("12,34.5"), None);
        assert_eq!(parse_value("40 (size: 3)"), None);
        assert_eq!(parse_value("1.2.3"), None);
    }

    #[test]
    fn test_strip_thousands_separators_only_with_spaced_list() {
        assert_eq!(strip_thousands_separators("1,000, 2,000"), "1000, 2000");
        assert_eq!(
            strip_thousands_separators("$1,000, $2,500.75"),
            "$1000, $2500.75"
        );
        assert_eq!(strip_thousands_separators("1,000,000, 5"), "1000000, 5");
        assert_eq!(strip_thousands_separators("10, 20, 30"), "10, 20, 30");
        // No comma+space anywhere: every comma separates items
        assert_eq!(strip_thousands_separators("100,200,300"), "100,200,300");
        assert_eq!(strip_thousands_separators("1,000"), "1,000");
        // Four digits after the comma is not a group
        assert_eq!(strip_thousands_separators("1,0000, 2"), "1,0000, 2");
    }

    #[test]
    fn test_parse_label_value() {
        assert_eq!(parse_label_value("Sales: 40"), Some(("Sales".into(), 40.0)));
        assert_eq!(
            parse_label_value("Revenue: $1,000"),
            Some(("Revenue".into(), 1000.0))
        );
        assert_eq!(
            parse_label_value("Share: 12%"),
            Some(("Share".into(), 12.0))
        );
        assert_eq!(
            parse_label_value("Load: 40 units"),
            Some(("Load".into(), 40.0))
        );
        assert_eq!(parse_label_value("Bad: inf"), None);
        assert_eq!(parse_label_value("no colon"), None);
        assert_eq!(parse_label_value("Empty: "), None);
    }
}
