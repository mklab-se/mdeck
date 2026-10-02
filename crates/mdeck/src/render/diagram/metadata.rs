use super::types::*;

// ─── Node metadata ───────────────────────────────────────────────────────────

/// A line with no metadata to parse.
fn bare(before: &str) -> NodeMetadata<'_> {
    NodeMetadata {
        before,
        icon: String::new(),
        grid_pos: None,
        prompt: None,
    }
}

/// Parse parenthetical metadata like `(icon: database, pos: 1,2, prompt: "...")`.
/// Returns the line content without the metadata and extracted fields.
pub(super) fn parse_node_metadata(s: &str) -> NodeMetadata<'_> {
    let trimmed = s.trim_end();
    if !trimmed.ends_with(')') {
        return bare(trimmed);
    }
    let Some(paren_start) = trimmed.rfind('(') else {
        return bare(trimmed);
    };
    // Only parse if there's whitespace before the paren
    if paren_start == 0 || trimmed.as_bytes()[paren_start - 1] != b' ' {
        return bare(trimmed);
    }

    let before = trimmed[..paren_start].trim_end();
    let meta_str = &trimmed[paren_start + 1..trimmed.len() - 1]; // contents between parens

    let mut icon = String::new();
    let mut grid_pos = None;
    let mut prompt = None;

    // Extract quoted prompt first (it may contain commas)
    let meta_str = extract_prompt(meta_str, &mut prompt);

    for part in meta_str.split(',') {
        let part = part.trim();
        if let Some(val) = part
            .strip_prefix("icon:")
            .or_else(|| part.strip_prefix("icon :"))
        {
            icon = val.trim().to_string();
        } else if let Some(val) = part
            .strip_prefix("pos:")
            .or_else(|| part.strip_prefix("pos :"))
        {
            let val = val.trim();
            // pos can be "x,y" but we already split on comma, so handle both forms
            if let Some((x_str, y_str)) = val.split_once(',') {
                if let (Ok(x), Ok(y)) = (x_str.trim().parse(), y_str.trim().parse()) {
                    grid_pos = Some((x, y));
                }
            } else if grid_pos.is_none() {
                // Might be split across commas: "pos: 1" then next part is "2"
                // Store x and look for y in next iteration
                if let Ok(x) = val.parse::<u32>() {
                    grid_pos = Some((x, 0)); // placeholder, y filled below
                }
            }
        } else if let Some((x, 0)) = grid_pos {
            // Continuation of pos value split by comma
            if let Ok(y) = part.trim().parse::<u32>() {
                grid_pos = Some((x, y));
            }
        }
    }

    NodeMetadata {
        before,
        icon,
        grid_pos,
        prompt,
    }
}

/// Extract a `prompt: "..."` or `prompt: '...'` value from the metadata string,
/// returning the remainder with the prompt portion removed.
fn extract_prompt(meta_str: &str, prompt: &mut Option<String>) -> String {
    // Look for prompt: followed by a quoted string
    let prefix = if let Some(idx) = meta_str.find("prompt:") {
        idx
    } else if let Some(idx) = meta_str.find("prompt :") {
        idx
    } else {
        return meta_str.to_string();
    };

    let after_key = &meta_str[prefix..];
    let after_colon = after_key
        .strip_prefix("prompt:")
        .or_else(|| after_key.strip_prefix("prompt :"))
        .unwrap_or(after_key);
    let after_colon = after_colon.trim_start();

    let (quote_char, rest) = if let Some(stripped) = after_colon.strip_prefix('"') {
        ('"', stripped)
    } else if let Some(stripped) = after_colon.strip_prefix('\'') {
        ('\'', stripped)
    } else {
        return meta_str.to_string();
    };

    if let Some(end) = rest.find(quote_char) {
        *prompt = Some(rest[..end].to_string());
        // Remove the prompt portion from the metadata string. `rest` is a
        // suffix of `meta_str`, which places the closing quote exactly.
        let prompt_end = meta_str.len() - rest.len() + end + 1;
        let mut result = meta_str[..prefix].to_string();
        if prompt_end < meta_str.len() {
            result.push_str(&meta_str[prompt_end..]);
        }
        result
    } else {
        meta_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata() {
        let meta = parse_node_metadata("Server (icon: server, pos: 2, 3)");
        assert_eq!(meta.before, "Server");
        assert_eq!(meta.icon, "server");
        assert_eq!(meta.grid_pos, Some((2, 3)));
        assert!(meta.prompt.is_none());
    }

    #[test]
    fn test_parse_metadata_with_prompt() {
        let meta =
            parse_node_metadata("Gateway (icon: generate:, prompt: \"An API gateway\", pos: 1, 1)");
        assert_eq!(meta.before, "Gateway");
        assert_eq!(meta.icon, "generate:");
        assert_eq!(meta.grid_pos, Some((1, 1)));
        assert_eq!(meta.prompt.as_deref(), Some("An API gateway"));
    }

    #[test]
    fn metadata_needs_a_space_before_the_paren() {
        let meta = parse_node_metadata("f(x)");
        assert_eq!(meta.before, "f(x)");
        assert!(meta.icon.is_empty());
        assert_eq!(parse_node_metadata("Plain  ").before, "Plain");
    }

    #[test]
    fn extract_prompt_removes_quoted_prompt() {
        let mut prompt = None;
        let rest = extract_prompt("icon: x, prompt:'a, b', pos: 1,2", &mut prompt);
        assert_eq!(prompt.as_deref(), Some("a, b"));
        assert_eq!(rest, "icon: x, , pos: 1,2");

        // A space after the colon (and `prompt :`) must not leave the
        // closing quote behind.
        let mut prompt = None;
        let rest = extract_prompt("icon: x, prompt: 'a b', pos: 1,2", &mut prompt);
        assert_eq!(prompt.as_deref(), Some("a b"));
        assert_eq!(rest, "icon: x, , pos: 1,2");
        let rest = extract_prompt("prompt :  \"c\", pos: 1,2", &mut prompt);
        assert_eq!(prompt.as_deref(), Some("c"));
        assert_eq!(rest, ", pos: 1,2");

        // An unquoted or unterminated prompt is left alone
        let mut prompt = None;
        assert_eq!(extract_prompt("prompt: bare", &mut prompt), "prompt: bare");
        assert_eq!(
            extract_prompt("prompt: \"open", &mut prompt),
            "prompt: \"open"
        );
        assert!(prompt.is_none());
    }
}
