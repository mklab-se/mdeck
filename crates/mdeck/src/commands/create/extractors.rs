//! File content extraction for PDF, DOCX, and text inputs.

use std::io;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

/// Extract text content from a file based on its extension.
pub fn extract_from_file(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" => extract_pdf(path),
        "docx" => extract_docx(path),
        _ => std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display())),
    }
}

/// Extract text from a PDF file.
fn extract_pdf(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read PDF: {}", path.display()))?;
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .with_context(|| format!("Failed to extract text from PDF: {}", path.display()))?;

    if text.trim().len() < 50 {
        eprintln!(
            "  {} PDF text extraction yielded very little content ({} chars).",
            "!".yellow().bold(),
            text.trim().len()
        );
        eprintln!("    The PDF may contain images or scanned text that cannot be extracted.");
    }

    Ok(text)
}

/// Extract text from a DOCX file.
fn extract_docx(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open DOCX: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read DOCX as ZIP: {}", path.display()))?;

    let mut doc_xml = String::new();
    {
        let mut doc_entry = archive
            .by_name("word/document.xml")
            .with_context(|| format!("No word/document.xml in DOCX: {}", path.display()))?;
        io::Read::read_to_string(&mut doc_entry, &mut doc_xml)?;
    }

    let text = extract_text_from_docx_xml(&doc_xml);

    if text.trim().len() < 50 {
        eprintln!(
            "  {} DOCX text extraction yielded very little content ({} chars).",
            "!".yellow().bold(),
            text.trim().len()
        );
    }

    Ok(text)
}

/// Extract plain text from DOCX XML content.
pub fn extract_text_from_docx_xml(xml: &str) -> String {
    let mut text = String::new();
    let mut in_text_tag = false;

    let mut chars = xml.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            for tc in chars.by_ref() {
                if tc == '>' {
                    break;
                }
                tag.push(tc);
            }

            let tag_trimmed = tag.trim();
            if tag_trimmed.starts_with("w:t") && !tag_trimmed.starts_with("w:tbl") {
                in_text_tag = !tag_trimmed.ends_with('/');
            } else if tag_trimmed == "/w:t" {
                in_text_tag = false;
            } else if tag_trimmed == "/w:p" {
                text.push('\n');
            }
        } else if in_text_tag {
            text.push(c);
        }
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docx_xml_basic_paragraph() {
        let xml = r#"<w:body><w:p><w:r><w:t>Hello world</w:t></w:r></w:p></w:body>"#;
        let text = extract_text_from_docx_xml(xml);
        assert_eq!(text.trim(), "Hello world");
    }

    #[test]
    fn test_docx_xml_multiple_paragraphs() {
        let xml = r#"<w:body><w:p><w:r><w:t>First</w:t></w:r></w:p><w:p><w:r><w:t>Second</w:t></w:r></w:p></w:body>"#;
        let text = extract_text_from_docx_xml(xml);
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines, vec!["First", "Second"]);
    }

    #[test]
    fn test_docx_xml_multiple_runs() {
        let xml = r#"<w:p><w:r><w:t>Hello </w:t></w:r><w:r><w:t>world</w:t></w:r></w:p>"#;
        let text = extract_text_from_docx_xml(xml);
        assert_eq!(text.trim(), "Hello world");
    }

    #[test]
    fn test_docx_xml_text_with_attributes() {
        let xml = r#"<w:p><w:r><w:t xml:space="preserve">Preserved text</w:t></w:r></w:p>"#;
        let text = extract_text_from_docx_xml(xml);
        assert_eq!(text.trim(), "Preserved text");
    }

    #[test]
    fn test_docx_xml_ignores_non_text_tags() {
        let xml = r#"<w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>Bold text</w:t></w:r></w:p>"#;
        let text = extract_text_from_docx_xml(xml);
        assert_eq!(text.trim(), "Bold text");
    }

    #[test]
    fn test_docx_xml_table_not_confused_with_text() {
        let xml = r#"<w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body>"#;
        let text = extract_text_from_docx_xml(xml);
        assert!(text.contains("Cell"));
    }

    #[test]
    fn test_docx_xml_empty_document() {
        let xml = r#"<w:body></w:body>"#;
        let text = extract_text_from_docx_xml(xml);
        assert!(text.trim().is_empty());
    }

    #[test]
    fn test_docx_xml_self_closing_text_tag() {
        let xml = r#"<w:p><w:r><w:t/>Outside text</w:r></w:p>"#;
        let text = extract_text_from_docx_xml(xml);
        assert!(!text.contains("Outside"));
    }
}
