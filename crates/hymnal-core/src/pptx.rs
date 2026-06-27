use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Raw text extracted from a single .pptx, before it becomes a HymnEntry.
pub struct ParsedPptx {
    pub number: Option<u32>,
    pub title: String,
    pub body: String,
}

/// Extract searchable text from a .pptx file.
///
/// The number is taken from the filename stem (authoritative; the in-slide
/// "Imnul N" text is unreliably split across XML runs). The title is the first
/// meaningful line of the slide with the lowest file index, skipping the
/// "Imnul ..." marker and "N/M" counter lines. The body is all slide text.
pub fn extract(path: &Path) -> Result<ParsedPptx> {
    let number = path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.trim_start_matches('0').parse::<u32>().ok())
        .or_else(|| {
            // stem like "000" trims to "" — treat as 0 only if all zeros
            path.file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| s.chars().all(|c| c == '0'))
                .map(|_| 0)
        });

    let file = std::fs::File::open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut zip = ZipArchive::new(file)
        .with_context(|| format!("read zip {}", path.display()))?;

    // Collect slide XML paths and sort by numeric index in the filename.
    let mut slide_names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();
    slide_names.sort_by_key(|n| slide_index(n));

    let mut all_lines: Vec<String> = Vec::new();
    let mut first_slide_lines: Vec<String> = Vec::new();
    for (idx, name) in slide_names.iter().enumerate() {
        let mut xml = String::new();
        zip.by_name(name)?.read_to_string(&mut xml)?;
        let lines = slide_text_lines(&xml)?;
        if idx == 0 {
            first_slide_lines = lines.clone();
        }
        all_lines.extend(lines);
    }

    let title = pick_title(&first_slide_lines);
    let body = all_lines.join("\n");
    Ok(ParsedPptx { number, title, body })
}

/// Numeric slide index from "ppt/slides/slide12.xml" -> 12.
fn slide_index(name: &str) -> u32 {
    name.trim_start_matches("ppt/slides/slide")
        .trim_end_matches(".xml")
        .parse()
        .unwrap_or(u32::MAX)
}

/// Extract `<a:t>` text runs from one slide's XML as trimmed non-empty lines.
fn slide_text_lines(xml: &str) -> Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut lines = Vec::new();
    let mut in_t = false;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.local_name().as_ref() == b"t" => in_t = true,
            Event::End(e) if e.local_name().as_ref() == b"t" => in_t = false,
            Event::Text(t) if in_t => {
                let s = t.unescape()?.trim().to_string();
                if !s.is_empty() {
                    lines.push(s);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(lines)
}

/// Pick the title: first line that is not the "Imnul" marker and not a "N/M"
/// counter, falling back to the first line of any kind.
fn pick_title(lines: &[String]) -> String {
    lines
        .iter()
        .find(|l| !is_marker(l))
        .or_else(|| lines.first())
        .cloned()
        .unwrap_or_default()
}

/// True for "Imnul ...", a bare number, or a "N/M" counter like "1/300".
fn is_marker(line: &str) -> bool {
    let l = line.trim();
    l.starts_with("Imnul")
        || l.chars().all(|c| c.is_ascii_digit())
        || (l.contains('/')
            && l.chars().all(|c| c.is_ascii_digit() || c == '/'))
}
