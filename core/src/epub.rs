use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineEntry {
    pub level: usize,
    pub title: String,
    pub chapter_idx: usize,
}
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EpubMetadata {
    pub title: String,
    pub authors: Vec<String>,
    pub series: Option<String>,
    pub series_index: Option<f32>,
    pub subjects: Vec<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub date: Option<String>,
    pub language: Option<String>,
    pub isbn: Option<String>,
}

impl EpubMetadata {
    pub fn primary_author(&self) -> Option<&str> {
        self.authors.first().map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpubCoverData {
    pub bytes: Vec<u8>,
    pub media_type: String,
    pub zip_path: String,
}

#[derive(Debug, Clone)]
pub struct EpubProbe {
    pub metadata: EpubMetadata,
    pub cover: Option<EpubCoverData>,
    pub chapter_count: usize,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpubRun {
    pub range: Range<usize>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub code: bool,
}

#[derive(Clone)]
pub enum EpubBlock {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
        runs: Vec<EpubRun>,
        is_quote: bool,
    },
    ListItem {
        text: String,
        runs: Vec<EpubRun>,
    },
    Image {
        alt: String,
        media_type: String,
        zip_path: String,
        bytes: Arc<Vec<u8>>,
    },
    Rule,
}

#[derive(Clone)]
pub struct EpubChapter {
    pub title: String,
    pub blocks: Vec<EpubBlock>,
}

#[derive(Clone)]
pub struct EpubTocItem {
    pub title: String,
    pub chapter_idx: usize,
    pub depth: u8,
}

#[derive(Clone)]
pub struct EpubBook {
    pub title: String,
    pub author: Option<String>,
    pub metadata: EpubMetadata,
    pub chapters: Vec<EpubChapter>,
    pub toc: Vec<EpubTocItem>,
    pub total_words: usize,
}

/// Estimated rendered characters per standard trade paperback book page (~275–300 words).
pub const CHARS_PER_BOOK_PAGE: f32 = 1750.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalBookLocation {
    pub page: usize,
    pub total_pages: usize,
    pub progress_frac: f32,
    pub progress_pct: u8,
}

pub fn chapter_book_weight(ch: &EpubChapter) -> f32 {
    let mut w = 0.0_f32;
    for blk in &ch.blocks {
        w += match blk {
            EpubBlock::Heading { text, .. } => text.chars().count() as f32 + 90.0,
            EpubBlock::Paragraph { text, .. } => text.chars().count() as f32 + 35.0,
            EpubBlock::ListItem { text, .. } => text.chars().count() as f32 + 25.0,
            EpubBlock::Image { .. } => 450.0,
            EpubBlock::Rule => 60.0,
        };
    }
    w.max(150.0)
}

impl EpubBook {
    pub fn chapter_cumulative_weights(&self) -> Vec<f32> {
        let mut cum = Vec::with_capacity(self.chapters.len() + 1);
        cum.push(0.0);
        for ch in &self.chapters {
            let next = cum.last().copied().unwrap_or(0.0) + chapter_book_weight(ch);
            cum.push(next);
        }
        cum
    }

    pub fn global_location_from_weights(
        cum: &[f32],
        chapter_idx: usize,
        chapter_frac: f32,
    ) -> GlobalBookLocation {
        if cum.len() <= 1 {
            return GlobalBookLocation {
                page: 1,
                total_pages: 1,
                progress_frac: 0.0,
                progress_pct: 0,
            };
        }
        let n = cum.len() - 1;
        let ci = chapter_idx.min(n - 1);
        let total_weight = cum[n].max(1.0);
        let total_pages = (total_weight / CHARS_PER_BOOK_PAGE).round().max(1.0) as usize;
        let ch_w = (cum[ci + 1] - cum[ci]).max(0.0);
        let pos_w = (cum[ci] + ch_w * chapter_frac.clamp(0.0, 1.0)).clamp(0.0, total_weight);
        let progress_frac = (pos_w / total_weight).clamp(0.0, 1.0);
        let page = if total_pages <= 1 {
            1
        } else {
            ((progress_frac * (total_pages - 1) as f32).round() as usize + 1).clamp(1, total_pages)
        };
        let progress_pct = (progress_frac * 100.0).round() as u8;
        GlobalBookLocation {
            page,
            total_pages,
            progress_frac,
            progress_pct,
        }
    }

    pub fn global_page_location(
        &self,
        chapter_idx: usize,
        chapter_frac: f32,
    ) -> GlobalBookLocation {
        let cum = self.chapter_cumulative_weights();
        Self::global_location_from_weights(&cum, chapter_idx, chapter_frac)
    }
}

pub fn image_media_type_for_path(path: &Path) -> Option<&'static str> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

pub fn extension_for_cover(media_type: &str, zip_path: &str) -> &'static str {
    let mt = media_type.trim().to_ascii_lowercase();
    let p = zip_path.trim().to_ascii_lowercase();
    if mt.contains("png") || p.ends_with(".png") {
        "png"
    } else if mt.contains("webp") || p.ends_with(".webp") {
        "webp"
    } else if mt.contains("gif") || p.ends_with(".gif") {
        "gif"
    } else if mt.contains("bmp") || p.ends_with(".bmp") {
        "bmp"
    } else if mt.contains("svg") || p.ends_with(".svg") {
        "svg"
    } else {
        "jpg"
    }
}

pub fn is_supported_image_name(zip_path: &str) -> bool {
    let lower = zip_path.trim().to_ascii_lowercase();
    matches!(
        Path::new(&lower).extension().and_then(|e| e.to_str()),
        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg")
    )
}

fn read_zip_text<R: Read + std::io::Seek>(zip: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut entry = zip.by_name(name).ok()?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).ok()?;
    Some(buf)
}

fn read_zip_bytes<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
) -> Option<Vec<u8>> {
    let mut entry = zip.by_name(name).ok()?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

fn read_zip_image<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
    zip_path: &str,
) -> Option<(String, String, Arc<Vec<u8>>)> {
    if !is_supported_image_name(zip_path) {
        return None;
    }
    let bytes = read_zip_bytes(zip, zip_path)?;
    if bytes.is_empty() {
        return None;
    }
    let media_type = image_media_type_for_path(Path::new(zip_path))
        .unwrap_or("image/jpeg")
        .to_string();
    Some((media_type, zip_path.to_string(), Arc::new(bytes)))
}

fn resolve_zip_relative(base_dir: &str, href: &str) -> String {
    let clean_href = href.split('#').next().unwrap_or(href);
    let decoded = percent_decode_str(clean_href).decode_utf8_lossy();
    let mut parts: Vec<String> = if decoded.starts_with('/') {
        Vec::new()
    } else {
        base_dir
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .map(ToString::to_string)
            .collect()
    };
    for seg in decoded.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s.to_string()),
        }
    }
    parts.join("/")
}

fn locate_opf_path<R: Read + std::io::Seek>(zip: &mut ZipArchive<R>) -> Result<String> {
    if let Some(cont_xml) = read_zip_text(zip, "META-INF/container.xml")
        && let Ok(cont_doc) = roxmltree::Document::parse(&cont_xml)
        && let Some(full_path) = cont_doc
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "rootfile")
            .and_then(|n| n.attribute("full-path"))
    {
        let decoded = percent_decode_str(full_path).decode_utf8_lossy();
        let trimmed = decoded.trim_start_matches('/').to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    for i in 0..zip.len() {
        if let Ok(f) = zip.by_index(i)
            && let Ok(fname) = f.name()
            && fname.to_ascii_lowercase().ends_with(".opf")
        {
            return Ok(fname.into_owned());
        }
    }
    anyhow::bail!("EPUB missing OPF package document")
}

fn replace_html_entities_numeric(xml: &str) -> String {
    // Strip DOCTYPE declarations (which often trigger DTD errors in roxmltree)
    let mut cleaned = String::with_capacity(xml.len());
    let mut rest = xml.strip_prefix('\u{feff}').unwrap_or(xml);
    while let Some(pos) = rest.find("<!DOCTYPE") {
        cleaned.push_str(&rest[..pos]);
        if let Some(end) = rest[pos..].find('>') {
            rest = &rest[pos + end + 1..];
        } else {
            break;
        }
    }
    cleaned.push_str(rest);

    const ENTITIES: &[(&str, &str)] = &[
        ("&nbsp;", "&#160;"),
        ("&mdash;", "&#8212;"),
        ("&ndash;", "&#8211;"),
        ("&hellip;", "&#8230;"),
        ("&lsquo;", "&#8216;"),
        ("&rsquo;", "&#8217;"),
        ("&ldquo;", "&#8220;"),
        ("&rdquo;", "&#8221;"),
        ("&laquo;", "&#171;"),
        ("&raquo;", "&#187;"),
        ("&copy;", "&#169;"),
        ("&reg;", "&#174;"),
        ("&trade;", "&#8482;"),
        ("&bull;", "&#8226;"),
        ("&middot;", "&#183;"),
        ("&times;", "&#215;"),
        ("&divide;", "&#247;"),
        ("&deg;", "&#176;"),
        ("&eacute;", "&#233;"),
        ("&egrave;", "&#232;"),
        ("&agrave;", "&#224;"),
        ("&uuml;", "&#252;"),
        ("&ouml;", "&#246;"),
        ("&auml;", "&#228;"),
        ("&ccedil;", "&#231;"),
        ("&szlig;", "&#223;"),
        ("&sect;", "&#167;"),
        ("&para;", "&#182;"),
        ("&dagger;", "&#8224;"),
        ("&Dagger;", "&#8225;"),
        ("&prime;", "&#8242;"),
        ("&Prime;", "&#8243;"),
    ];
    for &(from, to) in ENTITIES {
        if cleaned.contains(from) {
            cleaned = cleaned.replace(from, to);
        }
    }
    cleaned
}

fn collect_inline_text(node: roxmltree::Node<'_, '_>, out: &mut String) {
    if node.is_text() {
        if let Some(t) = node.text() {
            out.push_str(t);
        }
        return;
    }
    if node.tag_name().name().eq_ignore_ascii_case("br") {
        out.push('\n');
        return;
    }
    for ch in node.children() {
        collect_inline_text(ch, out);
    }
}

fn push_run_char_span(
    runs: &mut Vec<EpubRun>,
    range: Range<usize>,
    bold: bool,
    italic: bool,
    underline: bool,
    code: bool,
) {
    if !(bold || italic || underline || code) || range.start >= range.end {
        return;
    }
    if let Some(last) = runs.last_mut()
        && last.range.end == range.start
        && last.bold == bold
        && last.italic == italic
        && last.underline == underline
        && last.code == code
    {
        last.range.end = range.end;
        return;
    }
    runs.push(EpubRun {
        range,
        bold,
        italic,
        underline,
        code,
    });
}

fn collect_inline_styled_inner(
    node: roxmltree::Node<'_, '_>,
    out_text: &mut String,
    out_runs: &mut Vec<EpubRun>,
    bold: bool,
    italic: bool,
    underline: bool,
    code: bool,
) {
    if node.is_text() {
        if let Some(t) = node.text() {
            for ch in t.chars() {
                if ch.is_whitespace() {
                    if !out_text.is_empty() && !out_text.ends_with(' ') {
                        let start = out_text.len();
                        out_text.push(' ');
                        push_run_char_span(
                            out_runs,
                            start..out_text.len(),
                            bold,
                            italic,
                            underline,
                            code,
                        );
                    }
                } else {
                    let start = out_text.len();
                    out_text.push(ch);
                    push_run_char_span(
                        out_runs,
                        start..out_text.len(),
                        bold,
                        italic,
                        underline,
                        code,
                    );
                }
            }
        }
        return;
    }
    if !node.is_element() {
        for ch in node.children() {
            collect_inline_styled_inner(ch, out_text, out_runs, bold, italic, underline, code);
        }
        return;
    }
    let tag = node.tag_name().name().to_ascii_lowercase();
    match tag.as_str() {
        "style" | "script" | "img" | "image" => return,
        "br" => {
            if !out_text.is_empty() && !out_text.ends_with(' ') {
                out_text.push(' ');
            }
            return;
        }
        _ => {}
    }
    let next_bold = bold || matches!(tag.as_str(), "b" | "strong");
    let next_italic = italic || matches!(tag.as_str(), "i" | "em" | "cite" | "var");
    let next_underline = underline || matches!(tag.as_str(), "u" | "ins");
    let next_code = code || matches!(tag.as_str(), "code" | "tt" | "kbd" | "samp");
    for ch in node.children() {
        collect_inline_styled_inner(
            ch,
            out_text,
            out_runs,
            next_bold,
            next_italic,
            next_underline,
            next_code,
        );
    }
}

fn collect_inline_text_and_runs(node: roxmltree::Node<'_, '_>) -> (String, Vec<EpubRun>) {
    let mut text = String::new();
    let mut runs = Vec::new();
    collect_inline_styled_inner(node, &mut text, &mut runs, false, false, false, false);
    while text.ends_with(' ') {
        text.pop();
    }
    let max_len = text.len();
    for r in &mut runs {
        r.range.start = r.range.start.min(max_len);
        r.range.end = r.range.end.min(max_len);
    }
    runs.retain(|r| r.range.start < r.range.end);
    (text, runs)
}

fn is_generic_chapter_title(raw: &str, book_title: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower == book_title.trim().to_ascii_lowercase()
        || lower.starts_with("section ")
        || matches!(
            lower.as_str(),
            "unknown" | "untitled" | "index" | "document" | "titlepage"
        )
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_simple_html_fragment(raw: &str) -> String {
    if !raw.contains('<') {
        return normalize_ws(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => {
                if !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
                in_tag = true;
            }
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    normalize_ws(&out)
}

fn is_isbn_identifier(node: roxmltree::Node<'_, '_>, text: &str) -> bool {
    let scheme_is_isbn = node.attributes().any(|a| {
        a.name().eq_ignore_ascii_case("scheme") && a.value().trim().eq_ignore_ascii_case("isbn")
    });
    if scheme_is_isbn {
        return true;
    }
    let lower = text.trim().to_ascii_lowercase();
    if lower.starts_with("urn:isbn:") || lower.starts_with("isbn:") {
        return true;
    }
    false
}

fn clean_isbn_value(raw: &str) -> String {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("urn:isbn:") {
        trimmed[9..].trim().to_string()
    } else if lower.starts_with("isbn:") {
        trimmed[5..].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_opf_metadata(opf_doc: &roxmltree::Document<'_>, path: &Path) -> EpubMetadata {
    let meta_node = opf_doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("metadata"));

    let mut meta = EpubMetadata::default();
    let mut seen_authors: HashSet<String> = HashSet::new();
    let mut seen_subjects: HashSet<String> = HashSet::new();

    struct CollectionInfo {
        name: String,
        kind: Option<String>,
        pos: Option<f32>,
    }
    let mut collections: Vec<(String, CollectionInfo)> = Vec::new();
    let mut refines: Vec<(String, String, String)> = Vec::new();

    if let Some(mnode) = meta_node {
        for el in mnode.descendants().filter(|n| n.is_element()) {
            let tag = el.tag_name().name().to_ascii_lowercase();
            let raw_text = el.text().unwrap_or_default();
            match tag.as_str() {
                "title" if meta.title.is_empty() => {
                    let clean = normalize_ws(raw_text);
                    if !clean.is_empty() {
                        meta.title = clean;
                    }
                }
                "creator" => {
                    let clean = normalize_ws(raw_text);
                    if !clean.is_empty() && seen_authors.insert(clean.to_ascii_lowercase()) {
                        meta.authors.push(clean);
                    }
                }
                "subject" => {
                    for part in raw_text.split(',') {
                        let clean = normalize_ws(part);
                        if !clean.is_empty() && seen_subjects.insert(clean.to_ascii_lowercase()) {
                            meta.subjects.push(clean);
                        }
                    }
                }
                "description" if meta.description.is_none() => {
                    let clean = strip_simple_html_fragment(raw_text);
                    if !clean.is_empty() {
                        meta.description = Some(clean);
                    }
                }
                "publisher" if meta.publisher.is_none() => {
                    let clean = normalize_ws(raw_text);
                    if !clean.is_empty() {
                        meta.publisher = Some(clean);
                    }
                }
                "date" if meta.date.is_none() => {
                    let clean = normalize_ws(raw_text);
                    if !clean.is_empty() {
                        meta.date = Some(clean);
                    }
                }
                "language" if meta.language.is_none() => {
                    let clean = normalize_ws(raw_text);
                    if !clean.is_empty() {
                        meta.language = Some(clean);
                    }
                }
                "identifier" if meta.isbn.is_none() => {
                    let clean = normalize_ws(raw_text);
                    if !clean.is_empty() && is_isbn_identifier(el, &clean) {
                        let cleaned = clean_isbn_value(&clean);
                        if !cleaned.is_empty() {
                            meta.isbn = Some(cleaned);
                        }
                    }
                }
                "meta" => {
                    let name = el.attribute("name").unwrap_or_default().trim();
                    let prop = el.attribute("property").unwrap_or_default().trim();
                    let content = el.attribute("content").unwrap_or(raw_text).trim();
                    if name.eq_ignore_ascii_case("calibre:series") {
                        let clean = normalize_ws(content);
                        if !clean.is_empty() && meta.series.is_none() {
                            meta.series = Some(clean);
                        }
                    } else if name.eq_ignore_ascii_case("calibre:series_index") {
                        if meta.series_index.is_none()
                            && let Ok(v) = content.parse::<f32>()
                        {
                            meta.series_index = Some(v);
                        }
                    } else if prop == "belongs-to-collection" {
                        let clean = normalize_ws(raw_text);
                        if !clean.is_empty() {
                            let id = el.attribute("id").unwrap_or_default().to_string();
                            collections.push((
                                id,
                                CollectionInfo {
                                    name: clean,
                                    kind: None,
                                    pos: None,
                                },
                            ));
                        }
                    } else if let Some(ref_attr) = el.attribute("refines") {
                        let ref_id = ref_attr.trim().trim_start_matches('#').to_string();
                        if !ref_id.is_empty() && !prop.is_empty() {
                            refines.push((ref_id, prop.to_string(), normalize_ws(raw_text)));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if !collections.is_empty() {
        for (ref_id, prop, val) in refines {
            if let Some((_, col)) = collections.iter_mut().find(|(id, _)| !id.is_empty() && id == &ref_id) {
                if prop == "collection-type" {
                    col.kind = Some(val.to_ascii_lowercase());
                } else if prop == "group-position"
                    && let Ok(num) = val.parse::<f32>()
                {
                    col.pos = Some(num);
                }
            }
        }
        let chosen = collections
            .iter()
            .find(|(_, c)| c.kind.as_deref() == Some("series"))
            .or_else(|| collections.first());
        if let Some((_, col)) = chosen {
            if meta.series.is_none() {
                meta.series = Some(col.name.clone());
            }
            if meta.series_index.is_none() {
                meta.series_index = col.pos;
            }
        }
    }

    if meta.title.is_empty() {
        meta.title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("EPUB Book")
            .to_string();
    }

    meta
}

#[derive(Debug, Clone)]
struct CoverItemLocation {
    id: String,
    zip_path: String,
    media_type: String,
    href: String,
    manifest_node_range: Range<usize>,
}

fn find_cover_in_opf(
    opf_doc: &roxmltree::Document<'_>,
    opf_dir: &str,
) -> Option<CoverItemLocation> {
    let cover_meta_id = opf_doc
        .descendants()
        .find(|n| {
            n.is_element()
                && n.tag_name().name().eq_ignore_ascii_case("meta")
                && n.attribute("name")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("cover"))
        })
        .and_then(|n| n.attribute("content"))
        .map(|s| s.trim().to_string());

    let mut items: Vec<(CoverItemLocation, String)> = Vec::new();
    for item in opf_doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("item"))
    {
        if let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) {
            let media_type = item.attribute("media-type").unwrap_or("").to_string();
            let props = item.attribute("properties").unwrap_or("").to_string();
            items.push((
                CoverItemLocation {
                    id: id.to_string(),
                    zip_path: resolve_zip_relative(opf_dir, href),
                    media_type,
                    href: href.to_string(),
                    manifest_node_range: item.range(),
                },
                props,
            ));
        }
    }

    // Priority 1: EPUB 3 properties="cover-image"
    if let Some((loc, _)) = items
        .iter()
        .find(|(_, props)| props.split_whitespace().any(|p| p == "cover-image"))
    {
        return Some(loc.clone());
    }

    // Priority 2: EPUB 2 <meta name="cover" content="<id>"/>
    if let Some(cid) = cover_meta_id.as_deref()
        && let Some((loc, _)) = items.iter().find(|(loc, _)| loc.id == cid)
    {
        return Some(loc.clone());
    }

    // Priority 3: Heuristic image item with "cover" in id or href
    items
        .into_iter()
        .find(|(loc, _)| {
            loc.media_type.to_ascii_lowercase().starts_with("image/")
                && (loc.id.to_ascii_lowercase().contains("cover")
                    || loc.href.to_ascii_lowercase().contains("cover"))
        })
        .map(|(loc, _)| loc)
}

fn read_cover_from_zip<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
    opf_doc: &roxmltree::Document<'_>,
    opf_dir: &str,
) -> Option<EpubCoverData> {
    let loc = find_cover_in_opf(opf_doc, opf_dir)?;
    let bytes = read_zip_bytes(zip, &loc.zip_path)?;
    if bytes.is_empty() {
        return None;
    }
    Some(EpubCoverData {
        bytes,
        media_type: loc.media_type,
        zip_path: loc.zip_path,
    })
}

pub fn probe_epub_fast(path: &Path) -> Result<EpubProbe> {
    let file_size_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let mut zip = ZipArchive::new(file).context("Invalid EPUB zip archive")?;
    let opf_path = locate_opf_path(&mut zip)?;
    let opf_dir = opf_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let opf_xml = read_zip_text(&mut zip, &opf_path).context("Failed to read EPUB OPF")?;
    let opf_doc = roxmltree::Document::parse(&opf_xml).context("Invalid EPUB OPF XML")?;

    let metadata = parse_opf_metadata(&opf_doc, path);
    let cover = read_cover_from_zip(&mut zip, &opf_doc, opf_dir);
    let chapter_count = opf_doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("itemref"))
        .count();

    Ok(EpubProbe {
        metadata,
        cover,
        chapter_count,
        file_size_bytes,
    })
}

fn xml_escape_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn xml_escape_attr(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn extension_for_cover_mime(media_type: &str) -> &'static str {
    let mt = media_type.trim().to_ascii_lowercase();
    if mt.contains("png") {
        "png"
    } else if mt.contains("webp") {
        "webp"
    } else if mt.contains("gif") {
        "gif"
    } else if mt.contains("bmp") {
        "bmp"
    } else if mt.contains("svg") {
        "svg"
    } else {
        "jpg"
    }
}

fn updated_cover_href(old_href: &str, ext: &str) -> String {
    if let Some((dir, fname)) = old_href.rsplit_once('/') {
        let stem = fname.rsplit_once('.').map(|(s, _)| s).unwrap_or(fname);
        format!("{dir}/{stem}.{ext}")
    } else {
        let stem = old_href
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(old_href);
        format!("{stem}.{ext}")
    }
}

/// Atomically updates EPUB metadata and (optionally) cover image in-place inside `<opf>` and ZIP.
pub fn write_epub_metadata(
    path: &Path,
    meta: &EpubMetadata,
    new_cover: Option<(&[u8], &str)>,
) -> Result<()> {
    let orig_file =
        File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let mut archive = ZipArchive::new(orig_file).context("Invalid EPUB zip archive")?;
    let opf_path = locate_opf_path(&mut archive)?;
    let opf_dir = opf_path
        .rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default();
    let opf_xml = read_zip_text(&mut archive, &opf_path).context("Failed to read EPUB OPF")?;
    let opf_doc = roxmltree::Document::parse(&opf_xml).context("Invalid EPUB OPF XML")?;

    let pkg_node = opf_doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("package"))
        .context("EPUB OPF missing <package> element")?;
    let version = pkg_node.attribute("version").unwrap_or("2.0");
    let is_epub3 = version.trim().starts_with('3');

    let meta_node = opf_doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("metadata"))
        .context("EPUB OPF missing <metadata> element")?;
    let manifest_node = opf_doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("manifest"))
        .context("EPUB OPF missing <manifest> element")?;

    // Collect collection IDs we are replacing so we also drop their <meta refines="#id"> tags cleanly.
    let mut dropped_collection_ids: HashSet<String> = HashSet::new();
    for ch in meta_node.children().filter(|n| n.is_element()) {
        if ch.tag_name().name().eq_ignore_ascii_case("meta")
            && ch.attribute("property") == Some("belongs-to-collection")
            && let Some(id) = ch.attribute("id")
        {
            dropped_collection_ids.insert(id.trim().to_string());
        }
    }

    let replace_cover_meta = new_cover.is_some();
    let mut preserved_children: Vec<String> = Vec::new();
    for ch in meta_node.children().filter(|n| n.is_element()) {
        let tag = ch.tag_name().name().to_ascii_lowercase();
        let drop = match tag.as_str() {
            "title" | "creator" | "subject" | "description" | "publisher" | "date" | "language" => {
                true
            }
            "identifier" => {
                let raw_t = ch.text().unwrap_or_default();
                is_isbn_identifier(ch, raw_t)
            }
            "meta" => {
                let name = ch.attribute("name").unwrap_or_default().trim();
                let prop = ch.attribute("property").unwrap_or_default().trim();
                let refines_id = ch
                    .attribute("refines")
                    .map(|r| r.trim().trim_start_matches('#'));
                name.eq_ignore_ascii_case("calibre:series")
                    || name.eq_ignore_ascii_case("calibre:series_index")
                    || (replace_cover_meta && name.eq_ignore_ascii_case("cover"))
                    || prop == "belongs-to-collection"
                    || (matches!(prop, "collection-type" | "group-position")
                        && refines_id.is_some_and(|id| dropped_collection_ids.contains(id)))
            }
            _ => false,
        };
        if !drop {
            preserved_children.push(opf_xml[ch.range()].to_string());
        }
    }

    // Handle optional cover replacement in <manifest>
    let existing_cover = find_cover_in_opf(&opf_doc, &opf_dir);
    let mut old_cover_zip_path: Option<String> = None;
    let mut new_cover_zip_path: Option<String> = None;
    let mut cover_id_for_meta: Option<String> = None;
    let mut edits: Vec<(usize, usize, String)> = Vec::new();

    if let Some((_, media_type)) = new_cover {
        let ext = extension_for_cover_mime(media_type);
        let manifest_text = &opf_xml[manifest_node.range()];
        if let Some(cov) = existing_cover {
            let new_href = updated_cover_href(&cov.href, ext);
            let new_zpath = resolve_zip_relative(&opf_dir, &new_href);
            if cov.zip_path != new_zpath {
                old_cover_zip_path = Some(cov.zip_path.clone());
            }
            new_cover_zip_path = Some(new_zpath);
            cover_id_for_meta = Some(cov.id.clone());

            let props_attr = if is_epub3 {
                " properties=\"cover-image\""
            } else {
                ""
            };
            let new_item_xml = format!(
                "<item id=\"{}\" href=\"{}\" media-type=\"{}\"{}/>",
                xml_escape_attr(&cov.id),
                xml_escape_attr(&new_href),
                xml_escape_attr(media_type),
                props_attr
            );
            edits.push((
                cov.manifest_node_range.start,
                cov.manifest_node_range.end,
                new_item_xml,
            ));
        } else {
            let existing_ids: HashSet<String> = manifest_node
                .children()
                .filter(|n| n.is_element())
                .filter_map(|n| n.attribute("id").map(ToString::to_string))
                .collect();
            let mut cid = "bookworm-cover".to_string();
            let mut suffix = 2usize;
            while existing_ids.contains(&cid) {
                cid = format!("bookworm-cover-{suffix}");
                suffix += 1;
            }
            let new_href = format!("{cid}.{ext}");
            let new_zpath = resolve_zip_relative(&opf_dir, &new_href);
            new_cover_zip_path = Some(new_zpath);
            cover_id_for_meta = Some(cid.clone());

            let props_attr = if is_epub3 {
                " properties=\"cover-image\""
            } else {
                ""
            };
            let new_item_xml = format!(
                "\n    <item id=\"{}\" href=\"{}\" media-type=\"{}\"{}/>\n  ",
                xml_escape_attr(&cid),
                xml_escape_attr(&new_href),
                xml_escape_attr(media_type),
                props_attr
            );
            let close_pos = manifest_text
                .rfind("</")
                .map(|rel| manifest_node.range().start + rel)
                .unwrap_or(manifest_node.range().end);
            edits.push((close_pos, close_pos, new_item_xml));
        }
    }

    // Preserve opening <metadata ...> attributes and ensure xmlns:dc / xmlns:opf exist if needed
    let has_dc_ns = meta_node
        .namespaces()
        .any(|ns| ns.name() == Some("dc"))
        || pkg_node.namespaces().any(|ns| ns.name() == Some("dc"));
    let has_opf_ns = meta_node
        .namespaces()
        .any(|ns| ns.name() == Some("opf"))
        || pkg_node.namespaces().any(|ns| ns.name() == Some("opf"));

    let meta_slice = &opf_xml[meta_node.range()];
    let open_end = meta_slice.find('>').unwrap_or(meta_slice.len());
    let mut open_tag = if meta_slice[..open_end].ends_with('/') {
        let mut s = meta_slice[..open_end - 1].to_string();
        s.push('>');
        s
    } else {
        format!("{}>", &meta_slice[..open_end])
    };

    if !has_dc_ns {
        open_tag.pop();
        open_tag.push_str(" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">");
    }
    if meta.isbn.as_ref().is_some_and(|s| !s.trim().is_empty()) && !is_epub3 && !has_opf_ns {
        open_tag.pop();
        open_tag.push_str(" xmlns:opf=\"http://www.idpf.org/2007/opf\">");
    }

    let close_tag = if let Some(rel) = meta_slice.rfind("</") {
        &meta_slice[rel..]
    } else {
        "</metadata>"
    };

    let mut inner_lines: Vec<String> = Vec::new();
    let title_clean = meta.title.trim();
    let final_title = if title_clean.is_empty() {
        "Untitled"
    } else {
        title_clean
    };
    inner_lines.push(format!("    <dc:title>{}</dc:title>", xml_escape_text(final_title)));

    for author in &meta.authors {
        let clean = author.trim();
        if !clean.is_empty() {
            inner_lines.push(format!("    <dc:creator>{}</dc:creator>", xml_escape_text(clean)));
        }
    }
    for subject in &meta.subjects {
        let clean = subject.trim();
        if !clean.is_empty() {
            inner_lines.push(format!("    <dc:subject>{}</dc:subject>", xml_escape_text(clean)));
        }
    }
    if let Some(desc) = meta.description.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        inner_lines.push(format!(
            "    <dc:description>{}</dc:description>",
            xml_escape_text(desc)
        ));
    }
    if let Some(pub_) = meta.publisher.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        inner_lines.push(format!(
            "    <dc:publisher>{}</dc:publisher>",
            xml_escape_text(pub_)
        ));
    }
    if let Some(date) = meta.date.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        inner_lines.push(format!("    <dc:date>{}</dc:date>", xml_escape_text(date)));
    }
    if let Some(lang) = meta.language.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        inner_lines.push(format!(
            "    <dc:language>{}</dc:language>",
            xml_escape_text(lang)
        ));
    }
    if let Some(isbn) = meta.isbn.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let cleaned = clean_isbn_value(isbn);
        if is_epub3 {
            inner_lines.push(format!(
                "    <dc:identifier id=\"bookworm-isbn\">urn:isbn:{}</dc:identifier>",
                xml_escape_text(&cleaned)
            ));
        } else {
            inner_lines.push(format!(
                "    <dc:identifier opf:scheme=\"ISBN\">{}</dc:identifier>",
                xml_escape_text(&cleaned)
            ));
        }
    }

    if let Some(series) = meta.series.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        inner_lines.push(format!(
            "    <meta name=\"calibre:series\" content=\"{}\"/>",
            xml_escape_attr(series)
        ));
        if let Some(ix) = meta.series_index {
            let ix_str = if (ix.fract()).abs() < 0.001 {
                format!("{:.0}", ix)
            } else {
                format!("{ix}")
            };
            inner_lines.push(format!(
                "    <meta name=\"calibre:series_index\" content=\"{}\"/>",
                xml_escape_attr(&ix_str)
            ));
            if is_epub3 {
                inner_lines.push(format!(
                    "    <meta property=\"belongs-to-collection\" id=\"bookworm-series\">{}</meta>",
                    xml_escape_text(series)
                ));
                inner_lines.push(
                    "    <meta refines=\"#bookworm-series\" property=\"collection-type\">series</meta>"
                        .to_string(),
                );
                inner_lines.push(format!(
                    "    <meta refines=\"#bookworm-series\" property=\"group-position\">{}</meta>",
                    xml_escape_text(&ix_str)
                ));
            }
        } else if is_epub3 {
            inner_lines.push(format!(
                "    <meta property=\"belongs-to-collection\" id=\"bookworm-series\">{}</meta>",
                xml_escape_text(series)
            ));
            inner_lines.push(
                "    <meta refines=\"#bookworm-series\" property=\"collection-type\">series</meta>"
                    .to_string(),
            );
        }
    }

    if let Some(cid) = cover_id_for_meta.as_deref() {
        inner_lines.push(format!(
            "    <meta name=\"cover\" content=\"{}\"/>",
            xml_escape_attr(cid)
        ));
    }

    for preserved in preserved_children {
        inner_lines.push(format!("    {}", preserved.trim()));
    }

    let new_metadata_xml = format!("{open_tag}\n{}\n  {close_tag}", inner_lines.join("\n"));
    edits.push((meta_node.range().start, meta_node.range().end, new_metadata_xml));

    // Apply edits from highest offset to lowest so indices remain exact
    edits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    let mut updated_opf_xml = opf_xml;
    for (start, end, replacement) in edits {
        updated_opf_xml.replace_range(start..end, &replacement);
    }

    // Write sibling temp zip atomically with uncompressed `mimetype` first
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let fname = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("book.epub");
    let tmp_path = parent.join(format!(".{fname}.bookworm-tmp-{}", std::process::id()));

    let write_result = (|| -> Result<()> {
        let out_file = File::create(&tmp_path)
            .with_context(|| format!("Failed to create temp EPUB {}", tmp_path.display()))?;
        let mut zw = zip::ZipWriter::new(out_file);

        // EPUB OCF spec requires `mimetype` first and uncompressed (Stored)
        let stored_opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zw.start_file("mimetype", stored_opts)?;
        zw.write_all(b"application/epub+zip")?;

        let deflated_opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let total_entries = archive.len();
        for idx in 0..total_entries {
            let mut entry = archive.by_index(idx)?;
            let name = entry.name()?.into_owned();
            let norm = name.trim_start_matches('/');
            if norm == "mimetype"
                || norm == opf_path
                || old_cover_zip_path.as_deref() == Some(norm)
                || new_cover_zip_path.as_deref() == Some(norm)
            {
                continue;
            }
            if entry.is_dir() {
                let dir_name = if name.ends_with('/') {
                    name
                } else {
                    format!("{name}/")
                };
                zw.add_directory(dir_name, deflated_opts)?;
                continue;
            }
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            zw.start_file(name, deflated_opts)?;
            zw.write_all(&buf)?;
        }

        zw.start_file(&opf_path, deflated_opts)?;
        zw.write_all(updated_opf_xml.as_bytes())?;

        if let (Some(cov_zpath), Some((cov_bytes, _))) = (new_cover_zip_path.as_deref(), new_cover) {
            zw.start_file(cov_zpath, deflated_opts)?;
            zw.write_all(cov_bytes)?;
        }

        let mut finished = zw.finish()?;
        finished.flush()?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err).context("Failed to replace EPUB file");
    }

    Ok(())
}

fn strip_html_tags_fallback(raw: &str) -> Vec<EpubBlock> {
    let mut blocks = Vec::new();
    let mut cur = String::new();
    let mut in_tag = false;
    let mut skip_style_script = false;
    let lower = raw.to_ascii_lowercase();
    let chars: Vec<char> = raw.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        if !in_tag && chars[i] == '<' {
            let tail: String = lower_chars[i..].iter().take(10).collect();
            if tail.starts_with("<style") || tail.starts_with("<script") {
                skip_style_script = true;
            } else if tail.starts_with("</style") || tail.starts_with("</script") {
                skip_style_script = false;
            } else if tail.starts_with("</p") || tail.starts_with("</div") || tail.starts_with("</h")
            {
                let t = normalize_ws(&cur);
                if !t.is_empty() {
                    blocks.push(EpubBlock::Paragraph {
                        text: t,
                        runs: Vec::new(),
                        is_quote: false,
                    });
                }
                cur.clear();
            }
            in_tag = true;
        } else if in_tag && chars[i] == '>' {
            in_tag = false;
        } else if !in_tag && !skip_style_script {
            cur.push(chars[i]);
        }
        i += 1;
    }
    let tail = normalize_ws(&cur);
    if !tail.is_empty() {
        blocks.push(EpubBlock::Paragraph {
            text: tail,
            runs: Vec::new(),
            is_quote: false,
        });
    }
    blocks
}

fn walk_xhtml_blocks<R: Read + std::io::Seek>(
    node: roxmltree::Node<'_, '_>,
    chap_dir: &str,
    in_quote: bool,
    zip: &mut ZipArchive<R>,
    blocks: &mut Vec<EpubBlock>,
) {
    if !node.is_element() {
        return;
    }
    let tag = node.tag_name().name().to_ascii_lowercase();
    match tag.as_str() {
        "style" | "script" | "head" => {}
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let lvl = tag[1..].parse::<u8>().unwrap_or(1).clamp(1, 6);
            let mut txt = String::new();
            collect_inline_text(node, &mut txt);
            let clean = normalize_ws(&txt);
            if !clean.is_empty() {
                blocks.push(EpubBlock::Heading {
                    level: lvl,
                    text: clean,
                });
            }
        }
        "hr" => blocks.push(EpubBlock::Rule),
        "img" | "image" => {
            let src = node
                .attributes()
                .find(|a| matches!(a.name(), "src" | "href"))
                .map(|a| a.value());
            if let Some(src_href) = src {
                let zpath = resolve_zip_relative(chap_dir, src_href);
                if let Some((media_type, zip_path, bytes)) = read_zip_image(zip, &zpath) {
                    let alt = node.attribute("alt").unwrap_or("").trim().to_string();
                    blocks.push(EpubBlock::Image {
                        alt,
                        media_type,
                        zip_path,
                        bytes,
                    });
                }
            }
        }
        "li" => {
            let (text, runs) = collect_inline_text_and_runs(node);
            if !text.is_empty() {
                blocks.push(EpubBlock::ListItem { text, runs });
            }
        }
        "blockquote" => {
            let has_inner_blocks = node.children().any(|c| {
                c.is_element()
                    && matches!(
                        c.tag_name().name().to_ascii_lowercase().as_str(),
                        "p" | "div" | "li"
                    )
            });
            if has_inner_blocks {
                for ch in node.children() {
                    walk_xhtml_blocks(ch, chap_dir, true, zip, blocks);
                }
            } else {
                let (text, runs) = collect_inline_text_and_runs(node);
                if !text.is_empty() {
                    blocks.push(EpubBlock::Paragraph {
                        text,
                        runs,
                        is_quote: true,
                    });
                }
            }
        }
        "p" => {
            // Check for embedded images inside `<p>` first
            for img_el in node.descendants().filter(|n| {
                n.is_element()
                    && matches!(
                        n.tag_name().name().to_ascii_lowercase().as_str(),
                        "img" | "image"
                    )
            }) {
                if let Some(src_href) = img_el
                    .attributes()
                    .find(|a| matches!(a.name(), "src" | "href"))
                    .map(|a| a.value())
                {
                    let zpath = resolve_zip_relative(chap_dir, src_href);
                    if let Some((media_type, zip_path, bytes)) = read_zip_image(zip, &zpath) {
                        let alt = img_el.attribute("alt").unwrap_or("").trim().to_string();
                        blocks.push(EpubBlock::Image {
                            alt,
                            media_type,
                            zip_path,
                            bytes,
                        });
                    }
                }
            }
            let (text, runs) = collect_inline_text_and_runs(node);
            if !text.is_empty() {
                blocks.push(EpubBlock::Paragraph {
                    text,
                    runs,
                    is_quote: in_quote,
                });
            }
        }
        _ => {
            for ch in node.children() {
                walk_xhtml_blocks(ch, chap_dir, in_quote, zip, blocks);
            }
        }
    }
}

fn collect_nav_toc(
    ol_node: roxmltree::Node<'_, '_>,
    nav_dir: &str,
    depth: u8,
    chapter_by_path: &HashMap<String, usize>,
    out: &mut Vec<EpubTocItem>,
) {
    for li in ol_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("li"))
    {
        if let Some(a) = li
            .children()
            .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("a"))
            && let Some(href) = a.attribute("href")
        {
            let target = resolve_zip_relative(nav_dir, href);
            if let Some(&ch_ix) = chapter_by_path.get(&target) {
                let mut lbl = String::new();
                collect_inline_text(a, &mut lbl);
                let title = normalize_ws(&lbl);
                if !title.is_empty() {
                    out.push(EpubTocItem {
                        title,
                        chapter_idx: ch_ix,
                        depth,
                    });
                }
            }
        }
        for sub_ol in li
            .children()
            .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("ol"))
        {
            collect_nav_toc(sub_ol, nav_dir, (depth + 1).min(6), chapter_by_path, out);
        }
    }
}

fn collect_ncx_toc(
    parent: roxmltree::Node<'_, '_>,
    ncx_dir: &str,
    depth: u8,
    chapter_by_path: &HashMap<String, usize>,
    out: &mut Vec<EpubTocItem>,
) {
    for np in parent
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "navPoint")
    {
        let label = np
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "navLabel")
            .map(|lbl| {
                let mut s = String::new();
                collect_inline_text(lbl, &mut s);
                normalize_ws(&s)
            })
            .unwrap_or_default();
        let src = np
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "content")
            .and_then(|c| c.attribute("src"));
        if let Some(href) = src {
            let target = resolve_zip_relative(ncx_dir, href);
            if let Some(&ch_ix) = chapter_by_path.get(&target)
                && !label.is_empty()
            {
                out.push(EpubTocItem {
                    title: label,
                    chapter_idx: ch_ix,
                    depth,
                });
            }
        }
        collect_ncx_toc(np, ncx_dir, (depth + 1).min(6), chapter_by_path, out);
    }
}

impl EpubBook {
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
        let mut zip = ZipArchive::new(file).context("Invalid EPUB zip archive")?;

        let opf_path = locate_opf_path(&mut zip)?;
        let opf_dir = opf_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let opf_xml = read_zip_text(&mut zip, &opf_path).context("Failed to read EPUB OPF")?;
        let opf_doc = roxmltree::Document::parse(&opf_xml).context("Invalid EPUB OPF XML")?;

        let metadata = parse_opf_metadata(&opf_doc, path);
        let title = metadata.title.clone();
        let author = metadata.primary_author().map(ToString::to_string);

        struct ManifestItem {
            path: String,
            media_type: String,
            properties: String,
        }
        let mut manifest: HashMap<String, ManifestItem> = HashMap::new();
        for item in opf_doc
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "item")
        {
            if let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) {
                manifest.insert(
                    id.to_string(),
                    ManifestItem {
                        path: resolve_zip_relative(opf_dir, href),
                        media_type: item.attribute("media-type").unwrap_or("").to_string(),
                        properties: item.attribute("properties").unwrap_or("").to_string(),
                    },
                );
            }
        }

        let mut spine_paths = Vec::new();
        for itemref in opf_doc
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "itemref")
        {
            if let Some(idref) = itemref.attribute("idref")
                && let Some(item) = manifest.get(idref)
            {
                spine_paths.push(item.path.clone());
            }
        }

        let mut chapter_by_path: HashMap<String, usize> = HashMap::new();
        let mut chapters = Vec::with_capacity(spine_paths.len());
        let mut total_words = 0usize;

        for (ix, ch_path) in spine_paths.iter().enumerate() {
            chapter_by_path.insert(ch_path.clone(), ix);
            let ch_dir = ch_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            let raw_html = read_zip_text(&mut zip, ch_path).unwrap_or_default();
            let pre = replace_html_entities_numeric(&raw_html);
            let mut blocks = Vec::new();
            let mut head_title: Option<String> = None;

            if let Ok(xhtml) = roxmltree::Document::parse(&pre) {
                head_title = xhtml
                    .descendants()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("title"))
                    .and_then(|n| n.text())
                    .map(normalize_ws)
                    .filter(|s| !s.is_empty());

                let root_el = xhtml
                    .descendants()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("body"))
                    .unwrap_or_else(|| xhtml.root_element());
                walk_xhtml_blocks(root_el, ch_dir, false, &mut zip, &mut blocks);
            } else {
                blocks = strip_html_tags_fallback(&raw_html);
            }

            let body_heading = blocks.iter().find_map(|b| match b {
                EpubBlock::Heading { text, .. } => Some(text.clone()),
                _ => None,
            });
            let resolved_title = body_heading
                .as_ref()
                .filter(|h| !is_generic_chapter_title(h, &title))
                .cloned()
                .or_else(|| {
                    head_title
                        .as_ref()
                        .filter(|t| !is_generic_chapter_title(t, &title))
                        .cloned()
                })
                .or(body_heading)
                .unwrap_or_else(|| format!("Section {}", ix + 1));

            for blk in &blocks {
                match blk {
                    EpubBlock::Heading { text, .. }
                    | EpubBlock::Paragraph { text, .. }
                    | EpubBlock::ListItem { text, .. } => {
                        total_words += text.split_whitespace().count();
                    }
                    _ => {}
                }
            }

            chapters.push(EpubChapter {
                title: resolved_title,
                blocks,
            });
        }

        // Parse TOC from EPUB3 nav first, else EPUB2 NCX
        let mut toc = Vec::new();
        if let Some(nav_item) = manifest
            .values()
            .find(|m| m.properties.split_whitespace().any(|p| p == "nav"))
            && let Some(nav_raw) = read_zip_text(&mut zip, &nav_item.path)
        {
            let nav_clean = replace_html_entities_numeric(&nav_raw);
            let nav_dir = nav_item.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            if let Ok(nav_doc) = roxmltree::Document::parse(&nav_clean)
                && let Some(nav_el) = nav_doc.descendants().find(|n| {
                    n.is_element()
                        && n.tag_name().name().eq_ignore_ascii_case("nav")
                        && n.attributes()
                            .any(|a| a.name() == "type" && a.value().contains("toc"))
                })
                && let Some(ol) = nav_el
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("ol"))
            {
                collect_nav_toc(ol, nav_dir, 1, &chapter_by_path, &mut toc);
            }
        }

        if toc.is_empty()
            && let Some(ncx_item) = manifest.values().find(|m| {
                m.media_type.contains("dtbncx") || m.path.to_ascii_lowercase().ends_with(".ncx")
            })
            && let Some(ncx_raw) = read_zip_text(&mut zip, &ncx_item.path)
        {
            let ncx_clean = replace_html_entities_numeric(&ncx_raw);
            let ncx_dir = ncx_item.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            if let Ok(ncx_doc) = roxmltree::Document::parse(&ncx_clean)
                && let Some(nav_map) = ncx_doc
                    .descendants()
                    .find(|n| n.is_element() && n.tag_name().name() == "navMap")
            {
                collect_ncx_toc(nav_map, ncx_dir, 1, &chapter_by_path, &mut toc);
            }
        }

        // Sync chapter titles from TOC if a chapter still has a generic/boilerplate title
        let mut seen_toc_chapters = HashSet::new();
        for item in &toc {
            if seen_toc_chapters.insert(item.chapter_idx)
                && !is_generic_chapter_title(&item.title, &title)
                && let Some(ch) = chapters.get_mut(item.chapter_idx)
                && is_generic_chapter_title(&ch.title, &title)
            {
                ch.title = item.title.clone();
            }
        }
        if toc.is_empty() {
            for (ix, ch) in chapters.iter().enumerate() {
                toc.push(EpubTocItem {
                    title: ch.title.clone(),
                    chapter_idx: ix,
                    depth: 1,
                });
            }
        }

        Ok(Self {
            title,
            author,
            metadata,
            chapters,
            toc,
            total_words,
        })
    }

    pub fn outline_headings(&self) -> Vec<OutlineEntry> {
        self.toc
            .iter()
            .map(|t| OutlineEntry {
                level: t.depth.clamp(1, 6) as usize,
                title: t.title.clone(),
                chapter_idx: t.chapter_idx,
            })
            .collect()
    }
}

/// Generates a clean two-chapter EPUB on disk for offline / first-run testing.
pub fn create_sample_epub(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut zw = zip::ZipWriter::new(file);

    zw.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    zw.write_all(b"application/epub+zip")?;

    let def = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zw.start_file("META-INF/container.xml", def)?;
    zw.write_all(
        br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
    )?;

    zw.start_file("OEBPS/content.opf", def)?;
    zw.write_all(
        br#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <metadata>
    <dc:title>Notes from the Signal</dc:title>
    <dc:creator>Bookworm Press</dc:creator>
    <dc:subject>Philosophy</dc:subject>
    <dc:subject>Craft</dc:subject>
    <meta name="calibre:series" content="Signal Trilogy"/>
    <meta name="calibre:series_index" content="1"/>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#,
    )?;

    zw.start_file("OEBPS/toc.ncx", def)?;
    zw.write_all(
        br#"<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <navMap>
    <navPoint id="n1" playOrder="1"><navLabel><text>I. Plain Files Outlive Platforms</text></navLabel><content src="ch1.xhtml"/></navPoint>
    <navPoint id="n2" playOrder="2"><navLabel><text>II. The Art of Margin Notes</text></navLabel><content src="ch2.xhtml"/></navPoint>
  </navMap>
</ncx>"#,
    )?;

    zw.start_file("OEBPS/ch1.xhtml", def)?;
    zw.write_all(
        br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Unknown</title></head><body>
<h1>I. Plain Files Outlive Platforms</h1>
<p>Every tool built around opaque databases eventually becomes a cage. Plain Markdown notes paired with open EPUB archives stay readable decades from now.</p>
<blockquote>A good reader disappears behind the sentence &mdash; select or tap any passage to highlight and attach a margin note.</blockquote>
<p>With Syncthing mirroring your <b>Library</b> across desktop, phone, and tablet, every bookmark, highlight color, and marginalia note merges seamlessly.</p>
</body></html>"#,
    )?;

    zw.start_file("OEBPS/ch2.xhtml", def)?;
    zw.write_all(
        br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Chapter Two</title></head><body>
<h1>II. The Art of Margin Notes</h1>
<p>Highlighting is only half of reading; writing a sharp two-sentence disagreement in the margin turns passive consumption into permanent craft.</p>
<ul>
  <li>Five highlight colors: Yellow, Green, Blue, Pink, and Purple.</li>
  <li>Four decoration styles: Highlight, Underline, Squiggly, and Strike.</li>
  <li>Kanagawa Dragon, AMOLED black, Sepia, and Paper reader themes.</li>
</ul>
</body></html>"#,
    )?;

    let mut finished = zw.finish()?;
    finished.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_sample_epub(path: &Path) {
        let file = File::create(path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        zw.start_file(
            "mimetype",
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zw.write_all(b"application/epub+zip").unwrap();
        zw.start_file("META-INF/container.xml", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(
            br#"<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
                <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
            </container>"#,
        )
        .unwrap();
        zw.start_file("OEBPS/content.opf", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(
            br#"<package xmlns="http://www.idpf.org/2007/opf" version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/" unique-identifier="uid">
                <metadata>
                    <dc:identifier id="uid">urn:uuid:1234-5678</dc:identifier>
                    <dc:title>Pocket Book</dc:title>
                    <dc:creator>Ada Lovelace</dc:creator>
                </metadata>
                <manifest>
                    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
                    <item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
                    <item id="c2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
                </manifest>
                <spine toc="ncx"><itemref idref="c1"/><itemref idref="c2"/></spine>
            </package>"#,
        )
        .unwrap();
        zw.start_file("OEBPS/toc.ncx", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(
            br#"<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/">
                <navMap>
                    <navPoint id="n1"><navLabel><text>Chapter One</text></navLabel><content src="ch1.xhtml#top"/></navPoint>
                    <navPoint id="n2"><navLabel><text>Chapter Two</text></navLabel><content src="ch2.xhtml"/></navPoint>
                </navMap>
            </ncx>"#,
        )
        .unwrap();
        zw.start_file("OEBPS/ch1.xhtml", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(
            br#"<!DOCTYPE html>
            <html xmlns="http://www.w3.org/1999/xhtml"><head><title>Unknown</title></head><body>
            <h1>Arrival</h1>
            <p>Hello&nbsp;<b>bold</b> <em>italic</em>&mdash;<code>fn()</code> indeed.</p>
            <blockquote>A wise quote.</blockquote>
            <ul><li>First <strong>point</strong></li></ul>
            </body></html>"#,
        )
        .unwrap();
        zw.start_file("OEBPS/ch2.xhtml", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(
            br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Unknown</title></head><body><p>Ending.</p></body></html>"#,
        )
        .unwrap();
        zw.finish().unwrap();
    }

    #[test]
    fn loads_epub_chapters_entities_and_toc() {
        let tmp = tempfile::tempdir().unwrap();
        let epub_path = tmp.path().join("book.epub");
        build_sample_epub(&epub_path);

        let book = EpubBook::load(&epub_path).unwrap();
        assert_eq!(book.title, "Pocket Book");
        assert_eq!(book.author.as_deref(), Some("Ada Lovelace"));
        assert_eq!(book.chapters.len(), 2);
        assert_eq!(book.chapters[0].title, "Arrival");
        assert_eq!(book.chapters[1].title, "Chapter Two");
        assert_eq!(book.toc.len(), 2);
        assert_eq!(book.outline_headings()[1].chapter_idx, 1);
        assert!(matches!(
            &book.chapters[0].blocks[0],
            EpubBlock::Heading { level: 1, text } if text == "Arrival"
        ));
        match &book.chapters[0].blocks[1] {
            EpubBlock::Paragraph { text, runs, is_quote: false } => {
                assert!(text.contains('\u{2014}'));
                assert!(runs.iter().any(|r| r.bold && &text[r.range.clone()] == "bold"));
                assert!(runs.iter().any(|r| r.italic && &text[r.range.clone()] == "italic"));
                assert!(runs.iter().any(|r| r.code && &text[r.range.clone()] == "fn()"));
            }
            _ => panic!("expected paragraph with runs"),
        }
        assert!(matches!(
            &book.chapters[0].blocks[2],
            EpubBlock::Paragraph { text, is_quote: true, .. } if text == "A wise quote."
        ));
        match &book.chapters[0].blocks[3] {
            EpubBlock::ListItem { text, runs } => {
                assert_eq!(text, "First point");
                assert!(runs.iter().any(|r| r.bold && &text[r.range.clone()] == "point"));
            }
            _ => panic!("expected list item"),
        }
    }

    #[test]
    fn probe_and_write_epub_metadata_and_cover_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let epub_path = tmp.path().join("book.epub");
        build_sample_epub(&epub_path);

        let initial = probe_epub_fast(&epub_path).unwrap();
        assert_eq!(initial.metadata.title, "Pocket Book");
        assert_eq!(initial.metadata.authors, vec!["Ada Lovelace"]);
        assert_eq!(initial.chapter_count, 2);
        assert!(initial.cover.is_none());

        let updated_meta = EpubMetadata {
            title: "Notes on the Engine".into(),
            authors: vec!["Ada Lovelace".into(), "Charles Babbage".into()],
            series: Some("Analytical Chronicles".into()),
            series_index: Some(1.0),
            subjects: vec!["Computing".into(), "History".into()],
            description: Some("A foundational treatise & benchmark.".into()),
            publisher: Some("Taylor & Francis".into()),
            date: Some("1843-08-01".into()),
            language: Some("en".into()),
            isbn: Some("978-0-12-345678-9".into()),
        };
        let png_bytes = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13];
        write_epub_metadata(&epub_path, &updated_meta, Some((&png_bytes, "image/png"))).unwrap();

        // Check ZIP OCF compliance: mimetype must be entry 0 and Stored
        {
            let file = File::open(&epub_path).unwrap();
            let mut zw = ZipArchive::new(file).unwrap();
            let first = zw.by_index(0).unwrap();
            assert_eq!(first.name().unwrap().as_ref(), "mimetype");
            assert_eq!(first.compression(), zip::CompressionMethod::Stored);
        }

        let probed = probe_epub_fast(&epub_path).unwrap();
        assert_eq!(probed.metadata, updated_meta);
        let cov = probed.cover.expect("cover should be stored in epub");
        assert_eq!(cov.media_type, "image/png");
        assert_eq!(cov.bytes, png_bytes);

        // Verify non-ISBN identifier and chapters survived
        let reloaded = EpubBook::load(&epub_path).unwrap();
        assert_eq!(reloaded.title, "Notes on the Engine");
        assert_eq!(reloaded.chapters.len(), 2);
        let file = File::open(&epub_path).unwrap();
        let mut zw = ZipArchive::new(file).unwrap();
        let opf = read_zip_text(&mut zw, "OEBPS/content.opf").unwrap();
        assert!(opf.contains("urn:uuid:1234-5678"));
    }

    #[test]
    fn global_book_location_weights_real_chapters_and_handles_frontmatter() {
        let para_len = 800usize;
        let make_ch = |title: &str, paras: usize| EpubChapter {
            title: title.into(),
            blocks: (0..paras)
                .map(|_| EpubBlock::Paragraph {
                    text: "x".repeat(para_len),
                    runs: Vec::new(),
                    is_quote: false,
                })
                .collect(),
        };
        let book = EpubBook {
            title: "Test Novel".into(),
            author: None,
            metadata: EpubMetadata::default(),
            chapters: vec![
                make_ch("Cover", 0),
                make_ch("Copyright", 0),
                make_ch("Dedication", 0),
                make_ch("Chapter 1", 60),
            ],
            toc: Vec::new(),
            total_words: 8000,
        };
        let fm = book.global_page_location(1, 0.5);
        assert!(fm.total_pages >= 20 && fm.total_pages <= 40);
        assert!(fm.progress_pct <= 5);
        assert_eq!(fm.page, 1);

        let mid = book.global_page_location(3, 0.5);
        assert!(mid.progress_pct >= 45 && mid.progress_pct <= 55);
        assert!(mid.page > 1 && mid.page < mid.total_pages);

        let end = book.global_page_location(3, 1.0);
        assert_eq!(end.page, end.total_pages);
        assert_eq!(end.progress_pct, 100);
    }
}
