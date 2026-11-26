pub mod dictionary;
pub mod epub;
pub mod export_md;
pub mod library;
#[cfg(feature = "online-metadata")]
pub mod metadata;

use anyhow::{Context, Result};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Returns true for hidden folders and build/ VCS directories ignored during scans.
pub fn is_ignored_dir(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.starts_with('.') {
        return true;
    }
    matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "node_modules" | "target" | "android" | ".obsidian" | ".git" | ".trash" | "build"
    )
}

/// Checks whether a path is an existing directory readable by the current process.
pub fn can_read_dir(path: &Path) -> bool {
    path.is_dir() && fs::read_dir(path).is_ok()
}

/// Normalizes a path relative to `root` using `/` separators.
pub fn normalize_rel_key(root: &Path, path: &Path) -> String {
    let raw = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    raw.trim_start_matches('/').to_string()
}

/// Atomically writes UTF-8 text to `path` via a sibling temporary file rename.
pub fn atomic_write_string(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dir {}", parent.display()))?;
    }
    let fname = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("state.json");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = parent.join(format!(".{fname}.bookworm-tmp-{}", std::process::id()));

    if fs::write(&tmp, content.as_bytes()).is_ok() {
        if fs::rename(&tmp, path).is_ok() {
            return Ok(());
        }
        let _ = fs::remove_file(&tmp);
    }

    fs::write(path, content.as_bytes())
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn decode_html_entity(ent: &str) -> Option<char> {
    match ent {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        "mdash" => Some('—'),
        "ndash" => Some('–'),
        "rsquo" => Some('’'),
        "lsquo" => Some('‘'),
        "rdquo" => Some('”'),
        "ldquo" => Some('“'),
        "hellip" => Some('…'),
        "copy" => Some('©'),
        "reg" => Some('®'),
        "trade" => Some('™'),
        _ if ent.starts_with("#x") || ent.starts_with("#X") => {
            u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32)
        }
        _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

/// Decodes common HTML/numeric entities in `s`.
pub fn decode_html_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'&'
            && let Some(semi_rel) = s[i + 1..].find(';')
            && semi_rel > 0
            && semi_rel <= 10
        {
            let ent = &s[i + 1..i + 1 + semi_rel];
            if let Some(ch) = decode_html_entity(ent) {
                out.push(ch);
                i += 1 + semi_rel + 1;
                continue;
            }
        }
        if let Some(ch) = s[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// Strips HTML tags and decodes entities, preserving line breaks at `<br>` and `</p>`.
pub fn strip_html_and_entities(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }
    let mut text_with_tags = String::with_capacity(raw.len());
    let mut chars = raw.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '<' {
            let tail = &raw[idx..];
            if let Some(gt) = tail.find('>') {
                let tag_body = tail[1..gt].trim().to_ascii_lowercase();
                if tag_body.starts_with("br") || tag_body.starts_with("/p") {
                    text_with_tags.push('\n');
                } else {
                    text_with_tags.push(' ');
                }
                while let Some(&(next_i, _)) = chars.peek() {
                    if next_i <= idx + gt {
                        chars.next();
                    } else {
                        break;
                    }
                }
                continue;
            }
        }
        text_with_tags.push(ch);
    }

    let decoded = decode_html_entities(&text_with_tags);
    decoded
        .lines()
        .map(|ln| ln.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|ln| !ln.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Lists direct non-hidden subdirectories of `dir`, sorted case-insensitively.
pub fn list_subdirectories(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut subdirs: Vec<(String, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            (p.is_dir() && !is_ignored_dir(&name)).then_some((name, p))
        })
        .collect();
    subdirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    subdirs
}

/// Counts regular files directly inside `dir` whose extension matches any entry in `exts`.
pub fn count_direct_matching_files(dir: &Path, exts: &[&str]) -> usize {
    let Ok(rd) = fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .filter(|e| {
            let path = e.path();
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                return false;
            };
            exts.iter().any(|allowed| ext.eq_ignore_ascii_case(allowed)) && path.is_file()
        })
        .count()
}

/// Finds all non-overlapping UTF-8 byte ranges in `haystack` matching `query`.
pub fn find_all_byte_ranges(haystack: &str, query: &str, case_sensitive: bool) -> Vec<Range<usize>> {
    if query.is_empty() || haystack.len() < (if case_sensitive { query.len() } else { 1 }) {
        return Vec::new();
    }
    if case_sensitive {
        let mut out = Vec::new();
        let mut pos = 0;
        while let Some(rel) = haystack[pos..].find(query) {
            let start = pos + rel;
            let end = start + query.len();
            out.push(start..end);
            pos = end;
        }
        return out;
    }
    if haystack.is_ascii() && query.is_ascii() {
        let hb = haystack.as_bytes();
        let qb = query.as_bytes();
        let qlen = qb.len();
        let mut out = Vec::new();
        let mut i = 0;
        while i + qlen <= hb.len() {
            if hb[i..i + qlen].eq_ignore_ascii_case(qb) {
                out.push(i..i + qlen);
                i += qlen;
            } else {
                i += 1;
            }
        }
        return out;
    }

    let q_chars: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    if q_chars.is_empty() {
        return Vec::new();
    }
    let mut lowered: Vec<(usize, usize, char)> = Vec::with_capacity(haystack.len());
    for (byte_start, ch) in haystack.char_indices() {
        let byte_end = byte_start + ch.len_utf8();
        for lc in ch.to_lowercase() {
            lowered.push((byte_start, byte_end, lc));
        }
    }
    let qlen = q_chars.len();
    let mut out = Vec::new();
    let mut k = 0;
    while k + qlen <= lowered.len() {
        let starts_char = k == 0 || lowered[k].0 != lowered[k - 1].0;
        let end_k = k + qlen;
        let ends_char = end_k == lowered.len() || lowered[end_k].0 != lowered[end_k - 1].0;
        if starts_char
            && ends_char
            && lowered[k..end_k]
                .iter()
                .zip(q_chars.iter())
                .all(|(a, b)| a.2 == *b)
        {
            out.push(lowered[k].0..lowered[end_k - 1].1);
            k = end_k;
        } else {
            k += 1;
        }
    }
    out
}

/// Merges base highlight styles with search match ranges into sorted non-overlapping spans.
pub fn merge_highlight_ranges<T: Clone>(
    base: &[(Range<usize>, T)],
    matches: &[Range<usize>],
    active_rel: Option<usize>,
    make_style: impl Fn(Option<&T>, bool) -> T,
) -> Vec<(Range<usize>, T)> {
    if matches.is_empty() {
        return base.to_vec();
    }
    let mut pts = Vec::with_capacity((base.len() + matches.len()) * 2);
    for (r, _) in base {
        if r.start < r.end {
            pts.push(r.start);
            pts.push(r.end);
        }
    }
    for r in matches {
        if r.start < r.end {
            pts.push(r.start);
            pts.push(r.end);
        }
    }
    if pts.len() < 2 {
        return Vec::new();
    }
    pts.sort_unstable();
    pts.dedup();

    let mut out = Vec::new();
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b {
            continue;
        }
        let base_style = base.iter().find(|(r, _)| r.start <= a && r.end >= b).map(|(_, s)| s);
        let match_idx = matches.iter().position(|r| r.start <= a && r.end >= b);
        if let Some(mix) = match_idx {
            let is_active = active_rel == Some(mix);
            out.push((a..b, make_style(base_style, is_active)));
        } else if let Some(s) = base_style {
            out.push((a..b, s.clone()));
        }
    }
    out
}
