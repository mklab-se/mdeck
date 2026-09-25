//! `$...$` inline and `$$...$$` display math in markdown text.
//!
//! The opening `$` must be followed, and the closing one preceded, by a
//! non-space, and a `$` followed by a letter or digit never closes. So
//! `$5 and $10` and `Revenue ($K) and cost ($M)` stay text, as does `\$`.
//! Backslashes inside a formula are LaTeX, not markdown escapes.

use super::Inline;

/// `$$...$$` or `$...$` starting at `start`: the inline and the index past it.
pub(super) fn try_math(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    if chars.get(start + 1).copied() == Some('$') {
        // display: $$ ... $$ (spaces allowed inside)
        let from = start + 2;
        let mut j = from;
        while j + 1 < chars.len() {
            if chars[j] == '\\' {
                j += 2;
                continue;
            }
            if chars[j] == '$' && chars[j + 1] == '$' {
                let tex: String = chars[from..j].iter().collect();
                let tex = tex.trim().to_string();
                if tex.is_empty() {
                    return None;
                }
                return Some((Inline::Math { tex, display: true }, j + 2));
            }
            j += 1;
        }
        return None;
    }
    // inline: $x$ with no space just inside either delimiter
    let first = chars.get(start + 1).copied()?;
    if first.is_whitespace() || first == '$' {
        return None;
    }
    let mut j = start + 1;
    while j < chars.len() {
        match chars[j] {
            '\\' => j += 2,
            '$' => {
                let closes = !chars[j - 1].is_whitespace()
                    && !chars
                        .get(j + 1)
                        .copied()
                        .is_some_and(|c| c.is_alphanumeric());
                if closes {
                    let tex: String = chars[start + 1..j].iter().collect();
                    return Some((
                        Inline::Math {
                            tex,
                            display: false,
                        },
                        j + 1,
                    ));
                }
                return None;
            }
            _ => j += 1,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::inline::parse;
    use super::*;

    fn math(tex: &str, display: bool) -> Inline {
        Inline::Math {
            tex: tex.into(),
            display,
        }
    }

    fn same(a: Vec<Inline>, b: Vec<Inline>) {
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    #[test]
    fn dollar_math_inline_and_display() {
        same(
            parse("Einstein: $E = mc^2$."),
            vec![
                Inline::Text("Einstein: ".into()),
                math("E = mc^2", false),
                Inline::Text(".".into()),
            ],
        );
        same(
            parse(r"$$ \frac{a}{b} $$"),
            vec![math(r"\frac{a}{b}", true)],
        );
        // backslashes inside math are LaTeX, not markdown escapes
        same(parse(r"$\alpha_1$"), vec![math(r"\alpha_1", false)]);
    }

    #[test]
    fn dollar_amounts_and_escapes_stay_text() {
        let text = |s: &str| vec![Inline::Text(s.into())];
        same(
            parse("Prices like $5 and $10 stay text."),
            text("Prices like $5 and $10 stay text."),
        );
        same(parse("a $ b $ c"), text("a $ b $ c"));
        same(
            parse("Revenue ($K) and cost ($M) both grew"),
            text("Revenue ($K) and cost ($M) both grew"),
        );
        same(
            parse("Pay $X per seat and $Y per site"),
            text("Pay $X per seat and $Y per site"),
        );
        same(parse(r"\$x\$"), text("$x$"));
        same(parse("$unclosed"), text("$unclosed"));
    }
}
