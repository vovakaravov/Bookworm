//! Online book metadata & cover lookup (Open Library + Apple Books Search API).

use crate::epub::EpubMetadata;
use crate::library::BookRecord;
use anyhow::{Context, Result, anyhow};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;
use std::collections::HashSet;
use std::io::Read as _;
use std::time::Duration;

const USER_AGENT: &str = "Bookworm/0.1 (book-metadata-matcher)";
const OL_SEARCH_FIELDS: &str =
    "key,title,author_name,first_publish_year,isbn,publisher,subject,cover_i,cover_edition_key,language";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchSource {
    OpenLibrary,
    AppleBooks,
    Merged,
}

impl MatchSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::OpenLibrary => "Open Library",
            Self::AppleBooks => "Apple Books",
            Self::Merged => "Open Library + Apple Books",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BookMatchCandidate {
    pub id: String,
    pub source: MatchSource,
    pub score: f32,
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
    pub cover_url: Option<String>,
    pub ol_work_key: Option<String>,
    pub ol_edition_key: Option<String>,
}

fn build_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .build()
        .into()
}

fn url_encode(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

fn has_cyrillic(s: &str) -> bool {
    s.chars().any(|ch| ('\u{0400}'..='\u{04FF}').contains(&ch))
}

fn strip_noisy_title_suffixes(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    loop {
        let low = s.to_ascii_lowercase();
        let mut stripped = None;
        for suffix in [
            "libgen.li",
            "royallib.com",
            "z-library",
            "pdfdrive.com",
            "(pdf)",
            ".epub",
            ".pdf",
            ".mobi",
            ".azw3",
            ".fb2",
        ] {
            if low.ends_with(suffix) {
                let cut = s.len() - suffix.len();
                let cand = s[..cut]
                    .trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '—' | '–'))
                    .trim()
                    .to_string();
                stripped = Some(cand);
                break;
            }
        }
        match stripped {
            Some(next) if !next.is_empty() && next != s => s = next,
            _ => break,
        }
    }
    s
}

/// Builds a clean search query from existing book metadata (`Title Author`).
pub fn default_search_query(meta: &EpubMetadata) -> String {
    let clean_title = strip_noisy_title_suffixes(&meta.title);
    let author = meta
        .primary_author()
        .map(str::trim)
        .filter(|a| !a.is_empty() && !a.eq_ignore_ascii_case("unknown author"));
    match (clean_title.is_empty(), author) {
        (false, Some(a)) => format!("{clean_title} {a}"),
        (false, None) => clean_title,
        (true, Some(a)) => a.to_string(),
        (true, None) => String::new(),
    }
}

/// Returns true when a book is missing core catalog fields or cover art.
pub fn needs_metadata_enrichment(rec: &BookRecord) -> bool {
    let missing_author = rec.metadata.authors.is_empty()
        || rec
            .metadata
            .authors
            .iter()
            .all(|a| a.trim().is_empty() || a.eq_ignore_ascii_case("unknown author"));
    let missing_desc = rec
        .metadata
        .description
        .as_deref()
        .is_none_or(|d| d.trim().len() < 40);
    let missing_isbn = rec
        .metadata
        .isbn
        .as_deref()
        .is_none_or(|i| i.trim().is_empty());
    let missing_pub = rec
        .metadata
        .publisher
        .as_deref()
        .is_none_or(|p| p.trim().is_empty());
    rec.cover.is_none()
        || missing_author
        || missing_desc
        || missing_isbn
        || missing_pub
        || rec.metadata.subjects.is_empty()
}

pub fn clean_isbn(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'x' || *c == 'X')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if cleaned.len() == 13 || cleaned.len() == 10 {
        Some(cleaned)
    } else {
        None
    }
}

fn pick_best_isbn(candidates: &[String]) -> Option<String> {
    let mut fallback10 = None;
    for raw in candidates {
        if let Some(code) = clean_isbn(raw) {
            if code.len() == 13 && (code.starts_with("978") || code.starts_with("979")) {
                return Some(code);
            }
            if code.len() == 13 && fallback10.as_ref().is_none_or(|s: &String| s.len() != 13) {
                fallback10 = Some(code);
            } else if fallback10.is_none() {
                fallback10 = Some(code);
            }
        }
    }
    fallback10
}

fn clean_author_name(raw: &str) -> Option<String> {
    let cleaned = raw
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(&[';', ','][..])
        .trim()
        .to_string();
    if cleaned.is_empty()
        || cleaned.eq_ignore_ascii_case("unknown")
        || cleaned.eq_ignore_ascii_case("unknown author")
        || cleaned.eq_ignore_ascii_case("auth.")
    {
        return None;
    }
    Some(cleaned)
}

pub fn clean_subject(raw: &str) -> Option<String> {
    let cleaned = raw
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| matches!(c, ' ' | '.' | ',' | '-' | ';' | '/'))
        .to_string();
    if cleaned.chars().count() < 3 || cleaned.chars().count() > 52 {
        return None;
    }
    let low = cleaned.to_ascii_lowercase();
    for bad in [
        "accessible book",
        "protected daisy",
        "in library",
        "overdrive",
        "internet ebook",
        "(pdf)",
        "kindle",
        "study guides",
        "study guide",
        "study aids",
        "test prep",
        "cliff's",
        "cliffsnotes",
        "http://",
        "https://",
        "examinations",
        "libgen",
        "royallib",
    ] {
        if low.contains(bad) {
            return None;
        }
    }
    if matches!(
        low.as_str(),
        "books" | "ebooks" | "ebook" | "general" | "unknown" | "xxxs"
    ) {
        return None;
    }
    if cleaned.matches('(').count() != cleaned.matches(')').count() {
        return None;
    }
    Some(cleaned)
}

fn merge_subject_lists(sources: impl IntoIterator<Item = String>, cap: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in sources {
        for piece in raw.split(',') {
            if let Some(clean) = clean_subject(piece)
                && seen.insert(clean.to_ascii_lowercase())
            {
                out.push(clean);
                if out.len() >= cap {
                    return out;
                }
            }
        }
    }
    out
}

fn decode_entity(ent: &str) -> Option<char> {
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

fn decode_html_entities(s: &str) -> String {
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
            if let Some(ch) = decode_entity(ent) {
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

pub fn strip_html_and_entities(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }
    // Insert breaks for block boundaries before stripping tags.
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

fn norm_tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|tok| !tok.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn is_stopword(tok: &str) -> bool {
    matches!(
        tok,
        "the" | "a" | "an" | "and" | "of" | "in" | "to" | "for" | "edition" | "novel" | "book"
    )
}

fn token_set_core(s: &str) -> HashSet<String> {
    let all: HashSet<String> = norm_tokens(s).into_iter().collect();
    let filtered: HashSet<String> = all.iter().filter(|t| !is_stopword(t)).cloned().collect();
    if filtered.is_empty() { all } else { filtered }
}

fn is_study_guide_noise(title: &str) -> bool {
    let low = title.to_ascii_lowercase();
    [
        "study guide",
        "summary of",
        "cliff's",
        "cliffsnotes",
        "instaread",
        "bookrags",
        "bright summaries",
    ]
    .iter()
    .any(|bad| low.contains(bad))
}

fn score_candidate(
    query: &str,
    cand_title: &str,
    cand_authors: &[String],
    has_desc: bool,
    has_cover: bool,
    has_isbn: bool,
    lang_hint: Option<&str>,
    cand_lang: Option<&str>,
) -> f32 {
    let q_core = token_set_core(query);
    let t_core = token_set_core(cand_title);
    if q_core.is_empty() || t_core.is_empty() {
        return 0.0;
    }
    if is_study_guide_noise(cand_title) && !query.to_ascii_lowercase().contains("guide") {
        return 0.0;
    }

    let mut combined_cand: HashSet<String> = t_core.clone();
    for a in cand_authors {
        for tok in norm_tokens(a) {
            combined_cand.insert(tok);
        }
    }

    // Precision & recall mix over query tokens vs candidate title+author tokens
    let q_hits_all = q_core.iter().filter(|t| combined_cand.contains(*t)).count();
    let title_hits = q_core.iter().filter(|t| t_core.contains(*t)).count();
    let q_recall = q_hits_all as f32 / q_core.len() as f32;
    let title_precision = title_hits as f32 / t_core.len() as f32;
    if title_hits == 0 || q_recall < 0.35 {
        return 0.0;
    }

    let mut score = q_recall * 3.2 + title_precision * 1.4;
    if has_desc {
        score += 0.35;
    }
    if has_cover {
        score += 0.25;
    }
    if has_isbn {
        score += 0.2;
    }
    if let (Some(ql), Some(cl)) = (lang_hint, cand_lang)
        && !ql.is_empty()
        && !cl.is_empty()
        && !ql.eq_ignore_ascii_case(cl)
    {
        score -= 1.0;
    }
    if has_cyrillic(query) && !has_cyrillic(cand_title) {
        score -= 1.8;
    }
    score
}

pub(crate) fn parse_open_library_search_json(root: &Value, query: &str) -> Vec<BookMatchCandidate> {
    let Some(docs) = root.get("docs").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (idx, doc) in docs.iter().enumerate() {
        let Some(title) = doc.get("title").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if title.is_empty() {
            continue;
        }
        let authors: Vec<String> = doc
            .get("author_name")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .filter_map(clean_author_name)
                    .take(4)
                    .collect()
            })
            .unwrap_or_default();

        let isbns: Vec<String> = doc
            .get("isbn")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .take(20)
                    .collect()
            })
            .unwrap_or_default();
        let isbn = pick_best_isbn(&isbns);

        let publisher = doc
            .get("publisher")
            .and_then(Value::as_array)
            .and_then(|arr| arr.iter().filter_map(Value::as_str).find(|s| !s.trim().is_empty()))
            .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
            .map(|s| s.chars().take(80).collect::<String>());

        let date = doc
            .get("first_publish_year")
            .and_then(Value::as_i64)
            .map(|y| y.to_string());

        let subjects = merge_subject_lists(
            doc.get("subject")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .take(16)
                        .map(ToString::to_string)
                        .collect()
                })
                .unwrap_or_else(Vec::new),
            7,
        );

        let work_key = doc.get("key").and_then(Value::as_str).map(ToString::to_string);
        let edition_key = doc
            .get("cover_edition_key")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let cover_url = doc
            .get("cover_i")
            .and_then(Value::as_i64)
            .map(|cid| format!("https://covers.openlibrary.org/b/id/{cid}-L.jpg"))
            .or_else(|| {
                edition_key
                    .as_deref()
                    .map(|ek| format!("https://covers.openlibrary.org/b/olid/{ek}-L.jpg"))
            });

        let score = score_candidate(
            query,
            title,
            &authors,
            false,
            cover_url.is_some(),
            isbn.is_some(),
            None,
            None,
        );
        if score < 1.6 {
            continue;
        }
        out.push(BookMatchCandidate {
            id: format!("ol:{}:{}", work_key.as_deref().unwrap_or("x"), idx),
            source: MatchSource::OpenLibrary,
            score,
            title: title.to_string(),
            authors,
            series: None,
            series_index: None,
            subjects,
            description: None,
            publisher,
            date,
            language: None,
            isbn,
            cover_url,
            ol_work_key: work_key,
            ol_edition_key: edition_key,
        });
    }
    out
}

pub(crate) fn parse_apple_books_search_json(
    root: &Value,
    query: &str,
    lang_hint: Option<&str>,
) -> Vec<BookMatchCandidate> {
    let Some(results) = root.get("results").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (idx, item) in results.iter().enumerate() {
        let Some(title) = item.get("trackName").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if title.is_empty() {
            continue;
        }
        let authors: Vec<String> = item
            .get("artistName")
            .and_then(Value::as_str)
            .and_then(clean_author_name)
            .map(|a| vec![a])
            .unwrap_or_default();

        let desc = item
            .get("description")
            .and_then(Value::as_str)
            .map(strip_html_and_entities)
            .filter(|d| !d.is_empty());

        let subjects = merge_subject_lists(
            item.get("genres")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect()
                })
                .unwrap_or_else(Vec::new),
            7,
        );

        let date = item
            .get("releaseDate")
            .and_then(Value::as_str)
            .map(|s| s.chars().take(10).collect::<String>())
            .filter(|s| !s.is_empty());

        let cover_url = item
            .get("artworkUrl100")
            .and_then(Value::as_str)
            .map(|u| u.replace("100x100bb", "600x600bb"));

        let track_id = item
            .get("trackId")
            .and_then(Value::as_i64)
            .map(|id| id.to_string())
            .unwrap_or_else(|| idx.to_string());

        let score = score_candidate(
            query,
            title,
            &authors,
            desc.is_some(),
            cover_url.is_some(),
            false,
            lang_hint,
            None,
        );
        if score < 1.6 {
            continue;
        }

        out.push(BookMatchCandidate {
            id: format!("ab:{track_id}"),
            source: MatchSource::AppleBooks,
            score,
            title: title.to_string(),
            authors,
            series: None,
            series_index: None,
            subjects,
            description: desc,
            publisher: None,
            date,
            language: lang_hint.map(ToString::to_string),
            isbn: None,
            cover_url,
            ol_work_key: None,
            ol_edition_key: None,
        });
    }
    out
}

fn author_surnames(authors: &[String]) -> HashSet<String> {
    authors
        .iter()
        .filter_map(|a| a.split_whitespace().last())
        .flat_map(|s| norm_tokens(s))
        .collect()
}

fn pair_similarity(a: &BookMatchCandidate, b: &BookMatchCandidate) -> Option<f32> {
    let ta = token_set_core(&a.title);
    let tb = token_set_core(&b.title);
    if ta.is_empty() || tb.is_empty() {
        return None;
    }
    let inter = ta.intersection(&tb).count();
    let union = ta.union(&tb).count();
    let title_jacc = inter as f32 / union.max(1) as f32;
    if title_jacc < 0.6 {
        return None;
    }
    let sa = author_surnames(&a.authors);
    let sb = author_surnames(&b.authors);
    let author_match = sa.is_empty() || sb.is_empty() || !sa.is_disjoint(&sb);
    if !author_match {
        return None;
    }
    Some(title_jacc + if !sa.is_disjoint(&sb) { 0.5 } else { 0.0 })
}

pub(crate) fn merge_provider_candidates(
    ol: Vec<BookMatchCandidate>,
    ab: Vec<BookMatchCandidate>,
) -> Vec<BookMatchCandidate> {
    let mut used_ol = HashSet::new();
    let mut used_ab = HashSet::new();
    let mut merged = Vec::new();

    // Pair highest-scoring matching works across Open Library and Apple Books.
    let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
    for (oi, oc) in ol.iter().enumerate() {
        for (ai, ac) in ab.iter().enumerate() {
            if let Some(sim) = pair_similarity(oc, ac) {
                pairs.push((sim + oc.score + ac.score, oi, ai));
            }
        }
    }
    pairs.sort_by(|x, y| y.0.total_cmp(&x.0));

    for (_, oi, ai) in pairs {
        if used_ol.contains(&oi) || used_ab.contains(&ai) {
            continue;
        }
        used_ol.insert(oi);
        used_ab.insert(ai);
        let oc = &ol[oi];
        let ac = &ab[ai];

        let title = if ac.title.len() <= 96 && !ac.title.is_empty() {
            ac.title.clone()
        } else {
            oc.title.clone()
        };
        let authors = if oc.authors.len() >= ac.authors.len() && !oc.authors.is_empty() {
            oc.authors.clone()
        } else {
            ac.authors.clone()
        };
        let subjects = merge_subject_lists(
            ac.subjects
                .iter()
                .cloned()
                .chain(oc.subjects.iter().cloned()),
            7,
        );
        let description = ac.description.clone().or_else(|| oc.description.clone());
        let cover_url = oc.cover_url.clone().or_else(|| ac.cover_url.clone());

        merged.push(BookMatchCandidate {
            id: format!("merged:{}+{}", oc.id, ac.id),
            source: MatchSource::Merged,
            score: oc.score.max(ac.score) + 1.2,
            title,
            authors,
            series: oc.series.clone().or_else(|| ac.series.clone()),
            series_index: oc.series_index.or(ac.series_index),
            subjects,
            description,
            publisher: oc.publisher.clone().or_else(|| ac.publisher.clone()),
            date: oc.date.clone().or_else(|| ac.date.clone()),
            language: oc.language.clone().or_else(|| ac.language.clone()),
            isbn: oc.isbn.clone().or_else(|| ac.isbn.clone()),
            cover_url,
            ol_work_key: oc.ol_work_key.clone(),
            ol_edition_key: oc.ol_edition_key.clone(),
        });
    }

    for (oi, oc) in ol.into_iter().enumerate() {
        if !used_ol.contains(&oi) {
            merged.push(oc);
        }
    }
    for (ai, ac) in ab.into_iter().enumerate() {
        if !used_ab.contains(&ai) {
            merged.push(ac);
        }
    }

    merged.sort_by(|a, b| b.score.total_cmp(&a.score));
    merged.truncate(10);
    merged
}

fn fetch_json_url(agent: &ureq::Agent, url: &str) -> Result<Value> {
    let mut resp = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("GET {url} failed"))?;
    let body = resp.body_mut().read_to_string()?;
    serde_json::from_str(&body).context("Invalid JSON response")
}

/// Searches Open Library & Apple Books in sequence and returns ranked, merged candidates.
pub fn search_book_metadata(
    query: &str,
    isbn_hint: Option<&str>,
    lang_hint: Option<&str>,
) -> Result<Vec<BookMatchCandidate>> {
    let trimmed = query.trim();
    anyhow::ensure!(!trimmed.is_empty(), "Empty book search query");

    let agent = build_agent();
    let effective_isbn = clean_isbn(trimmed).or_else(|| isbn_hint.and_then(clean_isbn));

    // 1. Open Library
    let mut ol_err = None;
    let mut ol_candidates = Vec::new();
    if let Some(isbn) = effective_isbn.as_deref() {
        let url = format!(
            "https://openlibrary.org/search.json?isbn={}&fields={}&limit=6",
            url_encode(isbn),
            OL_SEARCH_FIELDS
        );
        match fetch_json_url(&agent, &url) {
            Ok(json) => {
                ol_candidates = parse_open_library_search_json(&json, trimmed);
            }
            Err(e) => ol_err = Some(e),
        }
    }
    if ol_candidates.is_empty() {
        let url = format!(
            "https://openlibrary.org/search.json?q={}&fields={}&limit=8",
            url_encode(trimmed),
            OL_SEARCH_FIELDS
        );
        match fetch_json_url(&agent, &url) {
            Ok(json) => {
                ol_err = None;
                ol_candidates = parse_open_library_search_json(&json, trimmed);
            }
            Err(e) => ol_err = Some(e),
        }
    }

    // 2. Apple Books / iTunes Search API
    let is_ru = has_cyrillic(trimmed) || lang_hint == Some("ru");
    let countries: &[&str] = if is_ru { &["ru", "us"] } else { &["us"] };
    let mut ab_err = None;
    let mut ab_candidates = Vec::new();
    for country in countries {
        let url = format!(
            "https://itunes.apple.com/search?media=ebook&limit=8&term={}&country={country}",
            url_encode(trimmed)
        );
        match fetch_json_url(&agent, &url) {
            Ok(json) => {
                ab_err = None;
                ab_candidates = parse_apple_books_search_json(&json, trimmed, lang_hint);
                if !ab_candidates.is_empty() {
                    break;
                }
            }
            Err(e) => ab_err = Some(e),
        }
    }

    if ol_candidates.is_empty()
        && ab_candidates.is_empty()
        && let (Some(e1), Some(e2)) = (ol_err, ab_err)
    {
        return Err(anyhow!("Open Library & Apple Books failed: {e1}; {e2}"));
    }

    Ok(merge_provider_candidates(ol_candidates, ab_candidates))
}

/// Fetches Open Library Work & Edition details when description, subjects, ISBN, or publisher are missing.
pub fn enrich_candidate_details(cand: &BookMatchCandidate) -> BookMatchCandidate {
    let mut out = cand.clone();
    let agent = build_agent();

    if let Some(work_key) = cand.ol_work_key.as_deref()
        && (out.description.is_none() || out.subjects.len() < 3)
    {
        let url = format!("https://openlibrary.org{work_key}.json");
        if let Ok(json) = fetch_json_url(&agent, &url) {
            if out.description.is_none() {
                let raw_desc = match json.get("description") {
                    Some(Value::String(s)) => Some(s.as_str()),
                    Some(Value::Object(m)) => m.get("value").and_then(Value::as_str),
                    _ => None,
                };
                if let Some(raw) = raw_desc {
                    let clean = strip_html_and_entities(raw);
                    if !clean.is_empty() {
                        out.description = Some(clean);
                    }
                }
            }
            if let Some(arr) = json.get("subjects").and_then(Value::as_array) {
                let extra = arr
                    .iter()
                    .filter_map(Value::as_str)
                    .take(14)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                out.subjects = merge_subject_lists(out.subjects.into_iter().chain(extra), 7);
            }
        }
    }

    if let Some(ed_key) = cand.ol_edition_key.as_deref()
        && (out.isbn.is_none() || out.publisher.is_none() || out.description.is_none())
    {
        let url = format!("https://openlibrary.org/books/{ed_key}.json");
        if let Ok(json) = fetch_json_url(&agent, &url) {
            if out.isbn.is_none() {
                let mut pool = Vec::new();
                for key in ["isbn_13", "isbn_10"] {
                    if let Some(arr) = json.get(key).and_then(Value::as_array) {
                        pool.extend(arr.iter().filter_map(Value::as_str).map(ToString::to_string));
                    }
                }
                out.isbn = pick_best_isbn(&pool);
            }
            if out.publisher.is_none()
                && let Some(p) = json
                    .get("publishers")
                    .and_then(Value::as_array)
                    .and_then(|arr| arr.iter().filter_map(Value::as_str).next())
            {
                let clean = p.split_whitespace().collect::<Vec<_>>().join(" ");
                if !clean.is_empty() {
                    out.publisher = Some(clean.chars().take(80).collect());
                }
            }
            if out.date.is_none()
                && let Some(d) = json.get("publish_date").and_then(Value::as_str)
            {
                let trimmed = d.trim();
                if !trimmed.is_empty() {
                    out.date = Some(trimmed.chars().take(16).collect());
                }
            }
            if out.description.is_none() {
                let raw_desc = match json.get("description") {
                    Some(Value::String(s)) => Some(s.as_str()),
                    Some(Value::Object(m)) => m.get("value").and_then(Value::as_str),
                    _ => None,
                };
                if let Some(raw) = raw_desc {
                    let clean = strip_html_and_entities(raw);
                    if !clean.is_empty() {
                        out.description = Some(clean);
                    }
                }
            }
        }
    }

    out
}

pub(crate) fn detect_cover_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 12 {
        return None;
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else {
        None
    }
}

/// Downloads cover image bytes and verifies image magic header (`PNG`/`JPEG`/`WebP`/`GIF`).
pub fn fetch_cover_bytes(url: &str) -> Result<(Vec<u8>, String)> {
    const MAX_BYTES: usize = 8 * 1024 * 1024;
    let agent = build_agent();
    let resp = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("Cover download failed: {url}"))?;
    let mut reader = resp.into_body().into_reader().take((MAX_BYTES + 1) as u64);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    anyhow::ensure!(buf.len() <= MAX_BYTES, "Cover image exceeds 8 MB limit");
    anyhow::ensure!(buf.len() >= 2048, "Cover image too small ({} bytes)", buf.len());
    let mime = detect_cover_mime(&buf)
        .ok_or_else(|| anyhow!("Downloaded cover is not PNG/JPEG/WebP/GIF"))?;
    Ok((buf, mime.to_string()))
}

/// Merges a matched candidate into `EpubMetadata` (either filling gaps or overwriting non-empty matched fields).
pub fn merge_into_metadata(
    existing: &EpubMetadata,
    cand: &BookMatchCandidate,
    overwrite_existing: bool,
) -> EpubMetadata {
    let mut out = existing.clone();

    if overwrite_existing || out.title.trim().is_empty() || out.title.eq_ignore_ascii_case("untitled") {
        if !cand.title.trim().is_empty() {
            out.title = cand.title.trim().to_string();
        }
    }

    let missing_authors = out.authors.is_empty()
        || out
            .authors
            .iter()
            .all(|a| a.trim().is_empty() || a.eq_ignore_ascii_case("unknown author"));
    if (overwrite_existing || missing_authors) && !cand.authors.is_empty() {
        out.authors = cand.authors.clone();
    }

    if let Some(series) = cand.series.as_ref().filter(|s| !s.trim().is_empty())
        && (overwrite_existing || out.series.as_deref().is_none_or(|s| s.trim().is_empty()))
    {
        out.series = Some(series.clone());
    }
    if cand.series_index.is_some() && (overwrite_existing || out.series_index.is_none()) {
        out.series_index = cand.series_index;
    }

    if !cand.subjects.is_empty() && (overwrite_existing || out.subjects.is_empty()) {
        out.subjects = cand.subjects.clone();
    }

    if let Some(desc) = cand.description.as_ref().filter(|d| !d.trim().is_empty()) {
        let existing_short = out
            .description
            .as_deref()
            .is_none_or(|d| d.trim().len() < 40);
        if overwrite_existing || existing_short {
            out.description = Some(desc.clone());
        }
    }

    if let Some(pub_str) = cand.publisher.as_ref().filter(|p| !p.trim().is_empty())
        && (overwrite_existing || out.publisher.as_deref().is_none_or(|p| p.trim().is_empty()))
    {
        out.publisher = Some(pub_str.clone());
    }

    if let Some(date) = cand.date.as_ref().filter(|d| !d.trim().is_empty())
        && (overwrite_existing || out.date.as_deref().is_none_or(|d| d.trim().is_empty()))
    {
        out.date = Some(date.clone());
    }

    if let Some(lang) = cand.language.as_ref().filter(|l| !l.trim().is_empty())
        && (overwrite_existing
            || out
                .language
                .as_deref()
                .is_none_or(|l| l.trim().is_empty() || l.eq_ignore_ascii_case("und")))
    {
        out.language = Some(lang.clone());
    }

    if let Some(isbn) = cand.isbn.as_ref().filter(|i| !i.trim().is_empty())
        && (overwrite_existing || out.isbn.as_deref().is_none_or(|i| i.trim().is_empty()))
    {
        out.isbn = Some(isbn.clone());
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strip_html_and_entities_handles_markup_and_entities() {
        let raw = "<p>A &ldquo;classic&rdquo; tale&mdash;with <i>style</i>.<br/>Line&nbsp;2 &amp; more &#39;quotes&#39; &#x2026;</p>";
        let got = strip_html_and_entities(raw);
        assert_eq!(got, "A “classic” tale—with style .\nLine 2 & more 'quotes' …");
    }

    #[test]
    fn subject_cleaning_filters_noise() {
        let items = vec![
            "Protected DAISY".to_string(),
            "Science Fiction".to_string(),
            "science fiction".to_string(),
            "Books".to_string(),
            "Dystopian, Classics".to_string(),
            "Broken (MARC fragment".to_string(),
        ];
        let got = merge_subject_lists(items, 7);
        assert_eq!(got, vec!["Science Fiction", "Dystopian", "Classics"]);
    }

    #[test]
    fn isbn_cleaning_prefers_13() {
        let pool = vec![
            "0-451-52493-4".to_string(),
            "9780451524935".to_string(),
            "invalid".to_string(),
        ];
        assert_eq!(pick_best_isbn(&pool), Some("9780451524935".to_string()));
        assert_eq!(clean_isbn("0141036141"), Some("0141036141".to_string()));
    }

    #[test]
    fn parse_and_merge_ol_and_apple_json() {
        let ol_json = json!({
            "docs": [{
                "key": "/works/OL1168083W",
                "title": "Nineteen Eighty-Four",
                "author_name": ["George Orwell"],
                "first_publish_year": 1949,
                "isbn": ["0451524934", "9780451524935"],
                "publisher": ["Secker & Warburg"],
                "subject": ["Totalitarianism", "Dystopian", "Protected DAISY"],
                "cover_i": 7222246,
                "cover_edition_key": "OL7353617M"
            }]
        });
        let ab_json = json!({
            "results": [{
                "trackId": 12345,
                "trackName": "1984",
                "artistName": "George Orwell",
                "description": "<p>Winston Smith toes the Party line.</p>",
                "genres": ["Classics", "Books", "Fiction & Literature"],
                "releaseDate": "1949-06-08T07:00:00Z",
                "artworkUrl100": "https://is1-ssl.mzstatic.com/image/thumb/100x100bb.jpg"
            }]
        });
        let ol = parse_open_library_search_json(&ol_json, "Nineteen Eighty-Four George Orwell");
        let ab = parse_apple_books_search_json(&ab_json, "1984 George Orwell", Some("en"));
        assert_eq!(ol.len(), 1);
        assert_eq!(ab.len(), 1);
        assert_eq!(ol[0].isbn.as_deref(), Some("9780451524935"));
        assert_eq!(ol[0].publisher.as_deref(), Some("Secker & Warburg"));

        // Pair exact matching title+author across OL and Apple Books
        let ol2 = parse_open_library_search_json(&ol_json, "Nineteen Eighty-Four");
        let ab2 = parse_apple_books_search_json(
            &json!({
                "results": [{
                    "trackId": 99,
                    "trackName": "Nineteen Eighty-Four",
                    "artistName": "George Orwell",
                    "description": "<p>Classic dystopia.</p>",
                    "genres": ["Classics", "Books"]
                }]
            }),
            "Nineteen Eighty-Four",
            Some("en"),
        );
        let merged = merge_provider_candidates(ol2, ab2);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, MatchSource::Merged);
        assert_eq!(merged[0].isbn.as_deref(), Some("9780451524935"));
        assert_eq!(merged[0].description.as_deref(), Some("Classic dystopia."));
        assert!(merged[0].subjects.contains(&"Classics".to_string()));
        assert!(merged[0].subjects.contains(&"Totalitarianism".to_string()));
    }

    #[test]
    fn merge_into_metadata_fill_vs_overwrite() {
        let existing = EpubMetadata {
            title: "My Custom Title".to_string(),
            authors: vec!["Existing Author".to_string()],
            series: Some("Book Series".to_string()),
            series_index: Some(2.0),
            subjects: vec![],
            description: None,
            publisher: Some("Original Press".to_string()),
            date: None,
            language: Some("UND".to_string()),
            isbn: None,
        };
        let cand = BookMatchCandidate {
            id: "m1".to_string(),
            source: MatchSource::Merged,
            score: 4.5,
            title: "Matched Title".to_string(),
            authors: vec!["Matched Author".to_string()],
            series: None,
            series_index: None,
            subjects: vec!["Sci-Fi".to_string()],
            description: Some("Long publisher blurb describing the book.".to_string()),
            publisher: Some("New Publisher".to_string()),
            date: Some("1965".to_string()),
            language: Some("en".to_string()),
            isbn: Some("9780441013593".to_string()),
            cover_url: None,
            ol_work_key: None,
            ol_edition_key: None,
        };

        let fill_only = merge_into_metadata(&existing, &cand, false);
        assert_eq!(fill_only.title, "My Custom Title");
        assert_eq!(fill_only.authors, vec!["Existing Author"]);
        assert_eq!(fill_only.publisher.as_deref(), Some("Original Press"));
        assert_eq!(fill_only.isbn.as_deref(), Some("9780441013593"));
        assert_eq!(fill_only.language.as_deref(), Some("en"));
        assert_eq!(fill_only.series.as_deref(), Some("Book Series"));

        let overwrite = merge_into_metadata(&existing, &cand, true);
        assert_eq!(overwrite.title, "Matched Title");
        assert_eq!(overwrite.authors, vec!["Matched Author"]);
        assert_eq!(overwrite.publisher.as_deref(), Some("New Publisher"));
        // Preserves existing series when candidate has None
        assert_eq!(overwrite.series.as_deref(), Some("Book Series"));
        assert_eq!(overwrite.series_index, Some(2.0));
    }

    #[test]
    fn detect_cover_mime_magic_bytes() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        png.extend_from_slice(&[0; 16]);
        assert_eq!(detect_cover_mime(&png), Some("image/png"));

        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xE0];
        jpg.extend_from_slice(&[0; 16]);
        assert_eq!(detect_cover_mime(&jpg), Some("image/jpeg"));

        let mut webp = b"RIFF1234WEBP".to_vec();
        webp.extend_from_slice(&[0; 8]);
        assert_eq!(detect_cover_mime(&webp), Some("image/webp"));

        assert_eq!(detect_cover_mime(b"<html>nope</html>"), None);
    }

    #[test]
    #[ignore = "Live network test against Open Library and Apple Books"]
    fn live_search_smoke_test() {
        let hits = search_book_metadata("Dune Frank Herbert", None, Some("en")).unwrap();
        assert!(!hits.is_empty());
        let detailed = enrich_candidate_details(&hits[0]);
        assert!(detailed.description.is_some() || detailed.isbn.is_some());
    }
}
