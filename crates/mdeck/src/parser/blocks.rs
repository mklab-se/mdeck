use super::{Block, Directive, ImageDirectives, Inline, ListItem, ListMarker};

/// Extract @ directives from the beginning of a slide's raw text.
/// Returns (directives, remaining content).
pub fn extract_directives(raw: &str) -> (Vec<Directive>, String) {
    let mut directives = Vec::new();
    let mut remaining_lines = Vec::new();
    let mut past_directives = false;

    for line in raw.lines() {
        if !past_directives {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Single-line HTML comments may precede directives
            if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
                continue;
            }
            if let Some(directive) = parse_directive_line(trimmed) {
                directives.push(directive);
                continue;
            }
            past_directives = true;
        }
        remaining_lines.push(line);
    }

    (directives, remaining_lines.join("\n"))
}

fn parse_directive_line(line: &str) -> Option<Directive> {
    if !line.starts_with('@') {
        return None;
    }
    let after_at = &line[1..];
    let colon_pos = after_at.find(':')?;
    let name = after_at[..colon_pos].trim().to_string();
    let value = after_at[colon_pos + 1..].trim().to_string();

    // Validate: name should be word characters and hyphens
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }

    Some(Directive { name, value })
}

/// Parse a slide's content string into a Vec<Block>.
pub fn parse(content: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Column separator: +++
        if trimmed == "+++" {
            blocks.push(Block::ColumnSeparator);
            i += 1;
            continue;
        }

        // HTML comment: <!-- ... --> (possibly spanning lines) — never rendered
        if trimmed.starts_with("<!--") {
            i = skip_html_comment(&lines, i);
            continue;
        }

        // Horizontal rule: *** or ___
        if is_horizontal_rule(trimmed) {
            blocks.push(Block::HorizontalRule);
            i += 1;
            continue;
        }

        // Heading: # ...
        if let Some(heading) = parse_heading(trimmed) {
            blocks.push(heading);
            i += 1;
            continue;
        }

        // Fenced code block: ``` or ~~~
        if is_fence_start(trimmed) {
            let fence_char = if trimmed.starts_with("```") { '`' } else { '~' };
            let (block, end) = parse_code_block(&lines, i, fence_char);
            blocks.push(block);
            i = end;
            continue;
        }

        // Image: ![alt](path)
        if let Some(img) = parse_image(trimmed) {
            blocks.push(img);
            i += 1;
            continue;
        }

        // Blockquote: > ...
        if is_blockquote_start(trimmed) {
            let (block, end) = parse_blockquote(&lines, i);
            blocks.push(block);
            i = end;
            continue;
        }

        // Table: | ... |
        if is_table_line(trimmed)
            && let Some((table, end)) = parse_table(&lines, i)
        {
            blocks.push(table);
            i = end;
            continue;
        }
        // Pipe lines that don't form a table fall through to a paragraph

        // Unordered list: - or + or *  (but not --- or ***)
        if is_list_start(trimmed) {
            let (block, end) = parse_list(&lines, i, false);
            blocks.push(block);
            i = end;
            continue;
        }

        // Ordered list: 1. ...
        if is_ordered_list_start(trimmed) {
            let (block, end) = parse_list(&lines, i, true);
            blocks.push(block);
            i = end;
            continue;
        }

        // Paragraph: collect consecutive non-blank, non-special lines
        let (block, end) = parse_paragraph(&lines, i);
        blocks.push(block);
        // parse_paragraph always consumes at least one line, so the loop
        // can never stall on a line no other branch accepted.
        i = end.max(i + 1);
    }

    blocks
}

fn is_fence_start(trimmed: &str) -> bool {
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn is_blockquote_start(trimmed: &str) -> bool {
    trimmed.starts_with("> ") || trimmed == ">"
}

fn is_table_line(trimmed: &str) -> bool {
    trimmed.starts_with('|') && trimmed.ends_with('|')
}

/// True if a (trimmed) line begins a block that a paragraph or a list item's
/// text must not swallow. Mirrors exactly what the main `parse` loop accepts,
/// so a line that isn't a real heading/image/etc. stays paragraph text.
fn is_block_start(trimmed: &str) -> bool {
    trimmed == "+++"
        || trimmed.starts_with("<!--")
        || is_horizontal_rule(trimmed)
        || parse_heading(trimmed).is_some()
        || is_fence_start(trimmed)
        || parse_image(trimmed).is_some()
        || is_blockquote_start(trimmed)
        || is_table_line(trimmed)
        || is_list_start(trimmed)
        || is_ordered_list_start(trimmed)
}

/// Skip an HTML comment starting at `lines[start]`. Returns the index of the
/// first line after the `-->` terminator (or `lines.len()` if unterminated).
fn skip_html_comment(lines: &[&str], start: usize) -> usize {
    let mut i = start;
    while i < lines.len() {
        // The opening `<!--` itself may be immediately followed by `-->`
        let hay = if i == start {
            lines[i].trim().get(4..).unwrap_or("")
        } else {
            lines[i]
        };
        i += 1;
        if hay.contains("-->") {
            break;
        }
    }
    i
}

fn is_horizontal_rule(line: &str) -> bool {
    let mut chars = line.chars().filter(|c| !c.is_whitespace());
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '*' && first != '_' {
        return false;
    }
    let mut count = 1;
    for c in chars {
        if c != first {
            return false;
        }
        count += 1;
    }
    count >= 3
}

fn parse_heading(line: &str) -> Option<Block> {
    if !line.starts_with('#') {
        return None;
    }

    let level = line.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }

    // `#` is ASCII so the byte index equals the char index here
    let rest = &line[level..];
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }

    let text = strip_closing_hashes(rest.trim());
    let inlines = super::inline::parse(text);
    Some(Block::Heading {
        level: level as u8,
        inlines,
    })
}

/// Remove an optional closing `#` sequence: `## Head ##` → `Head`.
/// Per CommonMark the closing run must be preceded by a space (`# C#` keeps
/// its hash), or the heading text must consist solely of hashes (`# ##` → ``).
fn strip_closing_hashes(text: &str) -> &str {
    let stripped = text.trim_end_matches('#');
    if stripped.is_empty() || stripped.ends_with(' ') {
        stripped.trim_end()
    } else {
        text
    }
}

/// Setext heading underline: `===` (H1) or `---` (H2), at least 3 chars.
fn setext_level(trimmed: &str) -> Option<u8> {
    if trimmed.len() < 3 {
        return None;
    }
    if trimmed.chars().all(|c| c == '=') {
        Some(1)
    } else if trimmed.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

fn parse_code_block(lines: &[&str], start: usize, fence_char: char) -> (Block, usize) {
    let opening = lines[start].trim();
    let fence_prefix: String = opening.chars().take_while(|&c| c == fence_char).collect();
    let fence_len = fence_prefix.len();

    // Parse language and highlight spec from opening line
    let after_fence = &opening[fence_len..];
    let (language, highlight_lines, viz_kind) = parse_code_info(after_fence.trim());

    let mut code_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        // Check for closing fence
        let closing_count = trimmed.chars().take_while(|&c| c == fence_char).count();
        if closing_count >= fence_len
            && trimmed
                .chars()
                .skip(closing_count)
                .all(|c| c.is_whitespace())
        {
            i += 1;
            break;
        }
        code_lines.push(lines[i]);
        i += 1;
    }

    let code = code_lines.join("\n");

    let block = match viz_kind {
        VizKind::Diagram => Block::Diagram { content: code },
        VizKind::WordCloud => Block::WordCloud { content: code },
        VizKind::Timeline => Block::Timeline { content: code },
        VizKind::PieChart => Block::PieChart { content: code },
        VizKind::BarChart => Block::BarChart { content: code },
        VizKind::LineChart => Block::LineChart { content: code },
        VizKind::DonutChart => Block::DonutChart { content: code },
        VizKind::KpiCards => Block::KpiCards { content: code },
        VizKind::FunnelChart => Block::FunnelChart { content: code },
        VizKind::RadarChart => Block::RadarChart { content: code },
        VizKind::StackedBar => Block::StackedBar { content: code },
        VizKind::VennDiagram => Block::VennDiagram { content: code },
        VizKind::ProgressBars => Block::ProgressBars { content: code },
        VizKind::ScatterPlot => Block::ScatterPlot { content: code },
        VizKind::OrgChart => Block::OrgChart { content: code },
        VizKind::GanttChart => Block::GanttChart { content: code },
        VizKind::GitGraph => Block::GitGraph { content: code },
        VizKind::None => Block::CodeBlock {
            language,
            code,
            highlight_lines,
        },
    };
    (block, i)
}

/// Which visualization type a code block represents (if any).
#[derive(Debug, Clone, Copy, PartialEq)]
enum VizKind {
    None,
    Diagram,
    WordCloud,
    Timeline,
    PieChart,
    BarChart,
    LineChart,
    DonutChart,
    KpiCards,
    FunnelChart,
    RadarChart,
    StackedBar,
    VennDiagram,
    ProgressBars,
    ScatterPlot,
    OrgChart,
    GanttChart,
    GitGraph,
}

fn parse_code_info(info: &str) -> (Option<String>, Vec<usize>, VizKind) {
    if info.is_empty() {
        return (None, vec![], VizKind::None);
    }

    // Check for visualization language tags
    if info.starts_with("@architecture") {
        return (None, vec![], VizKind::Diagram);
    }
    if info.starts_with("@wordcloud") {
        return (None, vec![], VizKind::WordCloud);
    }
    if info.starts_with("@timeline") {
        return (None, vec![], VizKind::Timeline);
    }
    if info.starts_with("@piechart") {
        return (None, vec![], VizKind::PieChart);
    }
    if info.starts_with("@barchart") {
        return (None, vec![], VizKind::BarChart);
    }
    if info.starts_with("@linechart") {
        return (None, vec![], VizKind::LineChart);
    }
    if info.starts_with("@donut") {
        return (None, vec![], VizKind::DonutChart);
    }
    if info.starts_with("@kpi") {
        return (None, vec![], VizKind::KpiCards);
    }
    if info.starts_with("@funnel") {
        return (None, vec![], VizKind::FunnelChart);
    }
    if info.starts_with("@radar") {
        return (None, vec![], VizKind::RadarChart);
    }
    if info.starts_with("@stackedbar") {
        return (None, vec![], VizKind::StackedBar);
    }
    if info.starts_with("@venn") {
        return (None, vec![], VizKind::VennDiagram);
    }
    if info.starts_with("@progress") {
        return (None, vec![], VizKind::ProgressBars);
    }
    if info.starts_with("@scatter") {
        return (None, vec![], VizKind::ScatterPlot);
    }
    if info.starts_with("@orgchart") {
        return (None, vec![], VizKind::OrgChart);
    }
    if info.starts_with("@gantt") {
        return (None, vec![], VizKind::GanttChart);
    }
    if info.starts_with("@gitgraph") {
        return (None, vec![], VizKind::GitGraph);
    }

    // Parse language and optional highlight spec.
    // The language is always the first whitespace-separated token, so
    // `rust title=x {1}` → "rust".
    let (lang_part, highlight_part) = if let Some(brace_start) = info.find('{') {
        let rest = &info[brace_start..];
        let highlight = if let Some(brace_end) = rest.find('}') {
            parse_highlight_spec(&rest[1..brace_end])
        } else {
            vec![]
        };
        (&info[..brace_start], highlight)
    } else {
        (info, vec![])
    };
    let lang_part = lang_part.split_whitespace().next().unwrap_or("");

    let language = if lang_part.is_empty() {
        None
    } else {
        Some(lang_part.to_string())
    };

    (language, highlight_part, VizKind::None)
}

/// Upper bound on the number of lines a single highlight range may span, so a
/// typo like `{1-99999999999}` can't allocate billions of entries.
const MAX_HIGHLIGHT_RANGE: usize = 10_000;

fn parse_highlight_spec(spec: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(s), Ok(e)) = (start.trim().parse::<usize>(), end.trim().parse::<usize>()) {
                if s > e {
                    continue;
                }
                let e = e.min(s.saturating_add(MAX_HIGHLIGHT_RANGE));
                lines.extend(s..=e);
            }
        } else if let Ok(n) = part.parse::<usize>() {
            lines.push(n);
        }
    }
    lines
}

fn parse_image(line: &str) -> Option<Block> {
    // ![alt](path)
    if !line.starts_with("![") {
        return None;
    }

    let close_bracket = line.find("](")?;
    let alt_full = &line[2..close_bracket];

    let paren_start = close_bracket + 2;
    let paren_end = line[paren_start..].find(')')? + paren_start;
    let path = image_path(&line[paren_start..paren_end]);

    // Extract directives from alt text
    let (alt, directives) = parse_image_alt(alt_full);

    Some(Block::Image {
        alt,
        path,
        directives,
    })
}

/// Extract the path from the inside of an image's parentheses.
///
/// Drops an optional title (`img.png "Title"` / `img.png 'Title'` /
/// `img.png (Title)`) and unwraps `<angle brackets>`. Unquoted paths that
/// simply contain spaces are kept whole.
fn image_path(inner: &str) -> String {
    let inner = inner.trim();
    if let Some(rest) = inner.strip_prefix('<')
        && let Some(end) = rest.find('>')
    {
        return rest[..end].to_string();
    }
    let mut parts = inner.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or("");
    let rest = parts.next().map(str::trim_start).unwrap_or("");
    if rest.starts_with('"') || rest.starts_with('\'') || rest.starts_with('(') {
        first.to_string()
    } else {
        inner.to_string()
    }
}

fn parse_image_alt(alt_full: &str) -> (String, ImageDirectives) {
    let mut directives = ImageDirectives::default();
    let mut alt_parts = Vec::new();

    for word in alt_full.split_whitespace() {
        if let Some(directive) = word.strip_prefix('@') {
            if directive == "fill" {
                directives.fill = true;
            } else if directive == "fit" {
                directives.fit = true;
            } else if directive == "left" {
                directives.align = Some("left".to_string());
            } else if directive == "right" {
                directives.align = Some("right".to_string());
            } else if directive == "center" {
                directives.align = Some("center".to_string());
            } else if let Some(val) = directive.strip_prefix("width:") {
                directives.width = Some(val.to_string());
            } else if let Some(val) = directive.strip_prefix("height:") {
                directives.height = Some(val.to_string());
            }
        } else {
            alt_parts.push(word);
        }
    }

    (alt_parts.join(" "), directives)
}

fn parse_blockquote(lines: &[&str], start: usize) -> (Block, usize) {
    let mut quote_text = String::new();
    let mut i = start;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if let Some(rest) = trimmed.strip_prefix("> ") {
            if !quote_text.is_empty() {
                quote_text.push(' ');
            }
            quote_text.push_str(rest);
            i += 1;
        } else if trimmed == ">" {
            if !quote_text.is_empty() {
                quote_text.push(' ');
            }
            i += 1;
        } else {
            break;
        }
    }

    let inlines = super::inline::parse(&quote_text);
    (Block::BlockQuote { inlines }, i)
}

/// Parse a table starting at `lines[start]`. Returns `None` (consuming
/// nothing) when the lines don't form a table: fewer than two lines, or the
/// second line is not a `|---|` separator row.
fn parse_table(lines: &[&str], start: usize) -> Option<(Block, usize)> {
    let mut table_lines: Vec<&str> = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with('|') {
            table_lines.push(trimmed);
            i += 1;
        } else if trimmed.is_empty() {
            i += 1;
            break;
        } else {
            break;
        }
    }

    if table_lines.len() < 2 || !is_table_separator(table_lines[1]) {
        return None;
    }

    // First line = headers
    let headers = parse_table_row(table_lines[0]);

    // Second line = separator (validated above)
    // Remaining lines = data rows
    let rows: Vec<Vec<Vec<Inline>>> = table_lines
        .iter()
        .skip(2)
        .map(|line| parse_table_row(line))
        .collect();

    Some((Block::Table { headers, rows }, i))
}

/// A separator row: every cell is `---`, `:--`, `--:` or `:-:` (1+ dashes).
fn is_table_separator(line: &str) -> bool {
    let cells = split_table_cells(line);
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let cell = cell.trim();
            let dashes = cell.trim_start_matches(':').trim_end_matches(':');
            !dashes.is_empty() && dashes.chars().all(|c| c == '-')
        })
}

fn parse_table_row(line: &str) -> Vec<Vec<Inline>> {
    split_table_cells(line)
        .iter()
        .map(|cell| super::inline::parse(cell.trim()))
        .collect()
}

/// Split a table row into raw cell strings, honouring `\|` (escaped pipe,
/// yielded as a literal `|`) and pipes inside backtick code spans. Leading and
/// trailing outer pipes are dropped.
fn split_table_cells(line: &str) -> Vec<String> {
    let line = line.trim();
    let chars: Vec<char> = line.chars().collect();
    let mut cells = Vec::new();
    let mut current = String::new();
    // Length of the backtick run that opened the current code span, if any
    let mut open_code: Option<usize> = None;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if chars.get(i + 1) == Some(&'|') => {
                current.push('|');
                i += 2;
            }
            '`' => {
                let run = chars[i..].iter().take_while(|&&c| c == '`').count();
                open_code = match open_code {
                    None => Some(run),
                    Some(n) if n == run => None,
                    other => other,
                };
                current.extend(std::iter::repeat_n('`', run));
                i += run;
            }
            '|' if open_code.is_none() => {
                cells.push(std::mem::take(&mut current));
                i += 1;
            }
            _ => {
                current.push(c);
                i += 1;
            }
        }
    }
    cells.push(current);

    // Drop the empty cells produced by the outer pipes
    if line.starts_with('|') && !cells.is_empty() {
        cells.remove(0);
    }
    if line.ends_with('|') && !line.ends_with("\\|") && cells.last().is_some_and(|c| c.is_empty()) {
        cells.pop();
    }
    cells
}

fn is_list_start(line: &str) -> bool {
    let mut chars = line.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some('-' | '+' | '*'), Some(' '))
    )
}

fn is_ordered_list_start(line: &str) -> bool {
    let Some(dot_pos) = line.find(". ") else {
        return false;
    };
    line[..dot_pos].trim().chars().all(|c| c.is_ascii_digit()) && dot_pos > 0
}

fn parse_list(lines: &[&str], start: usize, ordered: bool) -> (Block, usize) {
    let mut items: Vec<ListItem> = Vec::new();
    let mut i = start;
    // The first item's indent is the list's base level; deeper lines nest.
    let base_indent = line_indent(lines[start]);

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // Check if next non-blank line continues the list
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                let next = lines[j].trim();
                if is_list_start(next) || is_ordered_list_start(next) {
                    i = j;
                    continue;
                }
            }
            break;
        }

        let indent = line_indent(line);

        if indent <= base_indent {
            // Top-level item
            let item = if ordered {
                extract_ordered_item(trimmed)
            } else {
                extract_unordered_item(trimmed)
            };
            let Some((text, marker)) = item else {
                break;
            };
            i += 1;
            let text = collect_item_text(lines, &mut i, text);
            // Collect nested items
            let (children, new_i) = collect_children(lines, i, base_indent);
            items.push(ListItem {
                marker,
                inlines: super::inline::parse(&text),
                children,
            });
            i = new_i;
        } else {
            // A deeper-indented item after a blank line: nest it under the
            // last top-level item.
            let Some((text, marker)) = extract_any_list_item(trimmed) else {
                break;
            };
            i += 1;
            let text = collect_item_text(lines, &mut i, text);
            let (children, new_i) = collect_children(lines, i, indent);
            let item = ListItem {
                marker,
                inlines: super::inline::parse(&text),
                children,
            };
            match items.last_mut() {
                Some(last) => last.children.push(item),
                None => items.push(item),
            }
            i = new_i;
        }
    }

    (Block::List { ordered, items }, i)
}

/// Gather a list item's text: the marker line's text plus any following
/// continuation lines (wrapped or lazily indented text that is neither blank,
/// another list item, nor the start of another block). `i` is advanced past
/// the consumed lines.
fn collect_item_text(lines: &[&str], i: &mut usize, first: &str) -> String {
    let mut text = first.trim().to_string();
    while *i < lines.len() {
        let trimmed = lines[*i].trim();
        if trimmed.is_empty() || is_block_start(trimmed) {
            break;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(trimmed);
        *i += 1;
    }
    text
}

fn collect_children(lines: &[&str], start: usize, parent_indent: usize) -> (Vec<ListItem>, usize) {
    let mut children = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = line_indent(line);
        if indent <= parent_indent {
            break;
        }

        if let Some((text, marker)) = extract_any_list_item(trimmed) {
            i += 1;
            let text = collect_item_text(lines, &mut i, text);

            // Recursively collect deeper children
            let (sub_children, new_i) = collect_children(lines, i, indent);
            children.push(ListItem {
                marker,
                inlines: super::inline::parse(&text),
                children: sub_children,
            });
            i = new_i;
        } else {
            break;
        }
    }

    (children, i)
}

fn extract_unordered_item(line: &str) -> Option<(&str, ListMarker)> {
    if line.len() < 2 {
        return None;
    }
    let first = line.chars().next()?;
    let second = line.chars().nth(1)?;
    if second != ' ' {
        return None;
    }
    let marker = match first {
        '-' => ListMarker::Static,
        '+' => ListMarker::NextStep,
        '*' => ListMarker::WithPrev,
        _ => return None,
    };
    Some((&line[2..], marker))
}

fn extract_ordered_item(line: &str) -> Option<(&str, ListMarker)> {
    let dot_pos = line.find(". ")?;
    if dot_pos == 0 || !line[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((&line[dot_pos + 2..], ListMarker::Ordered))
}

fn extract_any_list_item(line: &str) -> Option<(&str, ListMarker)> {
    extract_unordered_item(line).or_else(|| extract_ordered_item(line))
}

fn line_indent(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

/// Parse a paragraph starting at `lines[start]`.
///
/// The first line is always consumed — even if it looks special but was
/// rejected by every other block parser (`#hashtag`, `![alt] text`) — so the
/// caller can never stall. Subsequent lines join until a blank line or the
/// start of another block. A one-line paragraph followed by a `===`/`---`
/// underline becomes a setext heading.
fn parse_paragraph(lines: &[&str], start: usize) -> (Block, usize) {
    let mut text = String::new();
    let mut i = start;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if i > start {
            // Setext heading: exactly one paragraph line + underline
            if i == start + 1
                && let Some(level) = setext_level(trimmed)
            {
                let inlines = super::inline::parse(&text);
                return (Block::Heading { level, inlines }, i + 1);
            }
            // Stop at blank lines or special block starts
            if trimmed.is_empty() || is_block_start(trimmed) {
                break;
            }
        }

        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(trimmed);
        i += 1;
    }

    let inlines = super::inline::parse(&text);
    (Block::Paragraph { inlines }, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_directives() {
        let raw = "@layout: two-column\n@theme: dark\n\n# Title\n\nContent";
        let (dirs, content) = extract_directives(raw);
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].name, "layout");
        assert_eq!(dirs[0].value, "two-column");
        assert_eq!(dirs[1].name, "theme");
        assert_eq!(dirs[1].value, "dark");
        assert!(content.contains("# Title"));
    }

    #[test]
    fn test_parse_heading() {
        let blocks = parse("# Title");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_code_block() {
        let blocks = parse("```rust\nfn main() {}\n```");
        assert_eq!(blocks.len(), 1);
        if let Block::CodeBlock { language, code, .. } = &blocks[0] {
            assert_eq!(language.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}");
        } else {
            panic!("Expected CodeBlock");
        }
    }

    #[test]
    fn test_parse_diagram_block() {
        let blocks = parse("```@architecture\n- A -> B: hello\n```");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::Diagram { .. }));
    }

    #[test]
    fn test_parse_image() {
        let blocks = parse("![Photo @fill](photo.jpg)");
        assert_eq!(blocks.len(), 1);
        if let Block::Image {
            alt,
            path,
            directives,
        } = &blocks[0]
        {
            assert_eq!(alt, "Photo");
            assert_eq!(path, "photo.jpg");
            assert!(directives.fill);
        } else {
            panic!("Expected Image");
        }
    }

    #[test]
    fn test_parse_image_width() {
        let blocks = parse("![Diagram @width:80%](diagram.png)");
        assert_eq!(blocks.len(), 1);
        if let Block::Image { directives, .. } = &blocks[0] {
            assert_eq!(directives.width.as_deref(), Some("80%"));
        } else {
            panic!("Expected Image");
        }
    }

    #[test]
    fn test_parse_blockquote() {
        let blocks = parse("> This is a quote\n> with multiple lines");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::BlockQuote { .. }));
    }

    #[test]
    fn test_parse_table() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let blocks = parse(input);
        assert_eq!(blocks.len(), 1);
        if let Block::Table { headers, rows } = &blocks[0] {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 1);
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let blocks = parse("- First\n- Second\n- Third");
        assert_eq!(blocks.len(), 1);
        if let Block::List { ordered, items } = &blocks[0] {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_parse_list_markers() {
        let blocks = parse("- Static\n+ Next\n* WithPrev");
        assert_eq!(blocks.len(), 1);
        if let Block::List { items, .. } = &blocks[0] {
            assert_eq!(items[0].marker, ListMarker::Static);
            assert_eq!(items[1].marker, ListMarker::NextStep);
            assert_eq!(items[2].marker, ListMarker::WithPrev);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let blocks = parse("Some text\n\n***\n\nMore text");
        assert!(blocks.iter().any(|b| matches!(b, Block::HorizontalRule)));
    }

    #[test]
    fn test_parse_column_separator() {
        let blocks = parse("Left content\n\n+++\n\nRight content");
        assert!(blocks.iter().any(|b| matches!(b, Block::ColumnSeparator)));
    }

    #[test]
    fn test_highlight_spec() {
        let result = parse_highlight_spec("3,5-7");
        assert_eq!(result, vec![3, 5, 6, 7]);
    }

    #[test]
    fn test_nested_list() {
        let blocks = parse("- Parent\n  - Child\n    - Grandchild");
        assert_eq!(blocks.len(), 1);
        if let Block::List { items, .. } = &blocks[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].children.len(), 1);
            assert_eq!(items[0].children[0].children.len(), 1);
        } else {
            panic!("Expected List");
        }
    }

    // --- Regression tests ---

    use crate::parser::inlines_to_text;

    fn paragraph_text(block: &Block) -> String {
        match block {
            Block::Paragraph { inlines } => inlines_to_text(inlines),
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    fn heading(block: &Block) -> (u8, String) {
        match block {
            Block::Heading { level, inlines } => (*level, inlines_to_text(inlines)),
            other => panic!("expected Heading, got {other:?}"),
        }
    }

    fn list_items(block: &Block) -> &Vec<ListItem> {
        match block {
            Block::List { items, .. } => items,
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn test_hash_lines_that_are_not_headings_do_not_hang() {
        // Each of these used to make `parse` loop forever: parse_heading
        // rejected the line and parse_paragraph refused to consume it.
        for input in ["#hashtag", "#include <stdio.h>", "#######", "#\tTitle", "#"] {
            let blocks = parse(input);
            assert_eq!(blocks.len(), 1, "input {input:?} → {blocks:?}");
        }
        assert_eq!(paragraph_text(&parse("#hashtag")[0]), "#hashtag");
        assert_eq!(paragraph_text(&parse("#######")[0]), "#######");
        assert_eq!(heading(&parse("#")[0]), (1, String::new()));

        // Inside a paragraph too
        let blocks = parse("Follow #rust on\n#mastodon today");
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            paragraph_text(&blocks[0]),
            "Follow #rust on #mastodon today"
        );
    }

    #[test]
    fn test_invalid_image_lines_do_not_hang() {
        for input in ["![alt] text", "![alt](path", "![", "![]"] {
            let blocks = parse(input);
            assert_eq!(blocks.len(), 1, "input {input:?} → {blocks:?}");
            assert!(matches!(blocks[0], Block::Paragraph { .. }));
        }
        let blocks = parse("line one\n![alt] text\nline three");
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            paragraph_text(&blocks[0]),
            "line one ![alt] text line three"
        );
    }

    #[test]
    fn test_single_multibyte_char_lines_do_not_panic() {
        for input in ["🎉", "→", "é", "→ x", "- 🎉", "**", "*", "_", "|", "~"] {
            let blocks = parse(input);
            assert_eq!(blocks.len(), 1, "input {input:?} → {blocks:?}");
        }
        assert!(!is_list_start("→"));
        assert!(!is_horizontal_rule("→"));
        assert!(!is_horizontal_rule("**"));
        assert!(is_horizontal_rule("* * *"));
        assert!(is_horizontal_rule("___"));
        assert!(is_list_start("- 🎉"));
        let blocks = parse("- 🎉 party");
        let items = list_items(&blocks[0]);
        assert_eq!(inlines_to_text(&items[0].inlines), "🎉 party");
        // Indentation is counted in characters, not bytes
        assert_eq!(line_indent("\t\tx"), 2);
    }

    #[test]
    fn test_highlight_spec_is_bounded() {
        let result = parse_highlight_spec("1-99999999999");
        assert_eq!(result.len(), MAX_HIGHLIGHT_RANGE + 1);
        assert_eq!(result[0], 1);
        // Reversed ranges are rejected, single lines and sane ranges still work
        assert_eq!(parse_highlight_spec("9-3"), Vec::<usize>::new());
        assert_eq!(parse_highlight_spec("3,5-7,x"), vec![3, 5, 6, 7]);
        assert_eq!(
            parse_highlight_spec("18446744073709551615-18446744073709551615").len(),
            1
        );
        let blocks = parse("```rust {1-99999999999}\nfn main() {}\n```");
        if let Block::CodeBlock {
            highlight_lines, ..
        } = &blocks[0]
        {
            assert_eq!(highlight_lines.len(), MAX_HIGHLIGHT_RANGE + 1);
        } else {
            panic!("Expected CodeBlock");
        }
    }

    #[test]
    fn test_code_info_language_is_first_token() {
        let blocks = parse("```rust title=x {1}\nfn main() {}\n```");
        if let Block::CodeBlock {
            language,
            highlight_lines,
            ..
        } = &blocks[0]
        {
            assert_eq!(language.as_deref(), Some("rust"));
            assert_eq!(highlight_lines, &vec![1]);
        } else {
            panic!("Expected CodeBlock");
        }
        let (lang, hl, _) = parse_code_info("python linenos");
        assert_eq!(lang.as_deref(), Some("python"));
        assert!(hl.is_empty());
        let (lang, hl, _) = parse_code_info("{2}");
        assert!(lang.is_none());
        assert_eq!(hl, vec![2]);
        let (lang, _, _) = parse_code_info("rust{3}");
        assert_eq!(lang.as_deref(), Some("rust"));
    }

    #[test]
    fn test_list_lazy_continuation_lines() {
        let blocks = parse("- First item that is long\n  and wraps here\n- Second");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let items = list_items(&blocks[0]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            inlines_to_text(&items[0].inlines),
            "First item that is long and wraps here"
        );
        assert_eq!(inlines_to_text(&items[1].inlines), "Second");

        // Unindented continuation
        let blocks = parse("- First\ncontinues\n- Second");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let items = list_items(&blocks[0]);
        assert_eq!(inlines_to_text(&items[0].inlines), "First continues");

        // Ordered lists
        let blocks = parse("1. Step one\n   more detail\n2. Step two\nlazy");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let items = list_items(&blocks[0]);
        assert_eq!(items.len(), 2);
        assert_eq!(inlines_to_text(&items[0].inlines), "Step one more detail");
        assert_eq!(inlines_to_text(&items[1].inlines), "Step two lazy");

        // Nested item continuation attaches to the nested item
        let blocks = parse("- Parent\n  - Child text\n    wrapped\n  - Sibling");
        let items = list_items(&blocks[0]);
        assert_eq!(items[0].children.len(), 2);
        assert_eq!(
            inlines_to_text(&items[0].children[0].inlines),
            "Child text wrapped"
        );

        // A real block start still ends the list
        let blocks = parse("- Item\n# Heading");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert!(matches!(blocks[1], Block::Heading { .. }));
        let blocks = parse("- Item\n\nParagraph");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
    }

    #[test]
    fn test_indented_list_keeps_items() {
        // A list whose first item is indented used to drop all its items
        let blocks = parse("  - a\n  - b\n    - c");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let items = list_items(&blocks[0]);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].children.len(), 1);
    }

    #[test]
    fn test_single_line_table_falls_back_to_paragraph() {
        let blocks = parse("| lonely |");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert_eq!(paragraph_text(&blocks[0]), "| lonely |");

        // Two pipe lines without a separator row are not a table either
        let blocks = parse("| a | b |\n| 1 | 2 |");
        assert!(blocks.iter().all(|b| matches!(b, Block::Paragraph { .. })));

        // But a valid table (with alignment colons) is
        let blocks = parse("| a | b |\n|:--|--:|\n| 1 | 2 |");
        assert!(matches!(blocks[0], Block::Table { .. }));
        assert!(is_table_separator("|---|:-:|"));
        assert!(is_table_separator("| --- | --- |"));
        assert!(!is_table_separator("| a | b |"));
        assert!(!is_table_separator("| --- | |"));
    }

    #[test]
    fn test_table_escaped_pipes_and_code_spans() {
        let input = "| Expr | Result |\n|---|---|\n| `a \\|\\| b` | x \\| y |\n| `c|d` | ``e|f`` |";
        let blocks = parse(input);
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        if let Block::Table { headers, rows } = &blocks[0] {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 2);
            assert!(matches!(&rows[0][0][0], Inline::Code(s) if s == "a || b"));
            assert_eq!(inlines_to_text(&rows[0][1]), "x | y");
            assert_eq!(rows[1].len(), 2);
            assert!(matches!(&rows[1][0][0], Inline::Code(s) if s == "c|d"));
            assert!(matches!(&rows[1][1][0], Inline::Code(s) if s == "e|f"));
        } else {
            panic!("Expected Table");
        }
        assert_eq!(split_table_cells("| a | | b |"), vec![" a ", " ", " b "]);
        assert_eq!(split_table_cells("a | b"), vec!["a ", " b"]);
    }

    #[test]
    fn test_setext_headings() {
        let blocks = parse("Title\n=====\n\nSub\n-----\n\ntext");
        assert_eq!(blocks.len(), 3, "{blocks:?}");
        assert_eq!(heading(&blocks[0]), (1, "Title".to_string()));
        assert_eq!(heading(&blocks[1]), (2, "Sub".to_string()));
        assert_eq!(paragraph_text(&blocks[2]), "text");

        // Multi-line paragraphs are not turned into headings
        let blocks = parse("one\ntwo\n===");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert!(matches!(blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_closing_hashes() {
        assert_eq!(heading(&parse("## Head ##")[0]), (2, "Head".to_string()));
        assert_eq!(
            heading(&parse("# Title #####")[0]),
            (1, "Title".to_string())
        );
        assert_eq!(heading(&parse("# C#")[0]), (1, "C#".to_string()));
        assert_eq!(heading(&parse("# ##")[0]), (1, String::new()));
    }

    #[test]
    fn test_html_comments_are_skipped() {
        let blocks = parse("<!-- hidden -->\n# Title\n\n<!--\nmulti\nline\n-->\n\nText");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert!(matches!(blocks[0], Block::Heading { .. }));
        assert_eq!(paragraph_text(&blocks[1]), "Text");

        // Comment interrupting a paragraph
        let blocks = parse("before\n<!-- note -->\nafter");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert_eq!(paragraph_text(&blocks[0]), "before");
        assert_eq!(paragraph_text(&blocks[1]), "after");

        // Unterminated comment swallows the rest without hanging
        let blocks = parse("<!-- oops\nText");
        assert!(blocks.is_empty(), "{blocks:?}");

        // Comments before directives don't end the directive prelude
        let (dirs, content) = extract_directives("<!-- c -->\n@layout: quote\n> q");
        assert_eq!(dirs.len(), 1);
        assert!(content.contains("> q"));
    }

    #[test]
    fn test_image_title_and_angle_brackets() {
        let blocks = parse("![alt](img.png \"Title\")");
        if let Block::Image { path, .. } = &blocks[0] {
            assert_eq!(path, "img.png");
        } else {
            panic!("Expected Image");
        }
        assert_eq!(image_path("img.png 'Title'"), "img.png");
        assert_eq!(image_path("img.png (Title)"), "img.png");
        assert_eq!(image_path("<my photo.png>"), "my photo.png");
        assert_eq!(image_path("<my photo.png> \"t\""), "my photo.png");
        // Unquoted paths with spaces are kept whole
        assert_eq!(image_path("my photo.png"), "my photo.png");
    }

    #[test]
    fn test_paragraph_breaks_only_on_real_blocks() {
        // Lines resembling block starts but rejected by their parser stay in
        // the paragraph; real block starts still end it.
        let blocks = parse("text\n![bad] img\n#tag\n> quote");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert_eq!(paragraph_text(&blocks[0]), "text ![bad] img #tag");
        assert!(matches!(blocks[1], Block::BlockQuote { .. }));
    }
}
