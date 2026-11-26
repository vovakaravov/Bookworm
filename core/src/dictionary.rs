//! Offline bilingual EN/RU SQLite dictionary with Russian morphology & online Wiktionary fallback.

use crate::strip_html_and_entities;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_DICT_PACK_URL: &str =
    "https://github.com/vovakaravov/Bookworm/releases/download/dict-v1/bookworm-dict-en-ru.db.gz";
pub const OFFLINE_DB_FILENAME: &str = "en-ru.db";
pub const CACHE_DB_FILENAME: &str = "user-cache.db";

const DICT_USER_AGENT: &str = "Bookworm/0.3 (bilingual-reader-dictionary)";
const LRU_CAPACITY: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookupSource {
    OfflinePack,
    CachedOnline,
    Online,
}

impl LookupSource {
    pub fn badge(self) -> &'static str {
        match self {
            Self::OfflinePack => "Offline",
            Self::CachedOnline => "Cached",
            Self::Online => "Online",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub lang: String,
    pub headword: String,
    pub stressed: String,
    pub ipa: String,
    pub pos: String,
    pub glosses: Vec<String>,
    pub ru_translation: Option<String>,
    pub matched_form: Option<String>,
    pub gram_tag: Option<String>,
    pub source: LookupSource,
}

impl DictionaryEntry {
    /// Returns the stressed Cyrillic form when available, otherwise `headword`.
    pub fn display_headword(&self) -> &str {
        if !self.stressed.is_empty() {
            &self.stressed
        } else {
            &self.headword
        }
    }

    /// Formats a clean note string suitable for attaching directly to a highlighted passage.
    pub fn format_as_note(&self) -> String {
        let mut header = self.display_headword().to_string();
        if !self.ipa.is_empty() {
            header.push_str(&format!(" [{}]", self.ipa));
        }
        if !self.pos.is_empty() {
            header.push_str(&format!(" ({})", self.pos));
        }
        let body = if self.glosses.is_empty() {
            String::new()
        } else {
            let joined = self
                .glosses
                .iter()
                .take(3)
                .map(|g| g.trim())
                .filter(|g| !g.is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            format!(" — {joined}")
        };
        let mut note = format!("{header}{body}");
        if let Some(ru) = &self.ru_translation
            && !ru.trim().is_empty()
        {
            note.push_str(&format!(" · RU: {}", ru.trim()));
        }
        note
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DictionaryStats {
    pub installed: bool,
    pub lemma_count: usize,
    pub form_count: usize,
    pub cached_count: usize,
    pub size_bytes: u64,
}

/// Extracts a clean 1–2 word lookup term from a reader selection, rejecting longer passages.
pub fn extract_lookup_word(selection: &str) -> Option<String> {
    let trimmed = selection
        .trim()
        .trim_matches(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '«' | '»'
                        | '"'
                        | '\''
                        | '’'
                        | '‘'
                        | '“'
                        | '”'
                        | '„'
                        | '('
                        | ')'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '.'
                        | ','
                        | ';'
                        | ':'
                        | '!'
                        | '?'
                        | '…'
                        | '—'
                        | '–'
                        | '-'
                )
        })
        .trim();
    if trimmed.is_empty() || trimmed.chars().count() > 48 {
        return None;
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.is_empty() || words.len() > 2 {
        return None;
    }
    if !trimmed.chars().any(char::is_alphabetic) {
        return None;
    }
    Some(words.join(" "))
}

pub fn detect_lang(word: &str) -> &'static str {
    if word.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)) {
        "ru"
    } else {
        "en"
    }
}

/// Lowercases, strips Russian combining acute stress marks (`\u{0301}`), and maps `ё` -> `е` for RU.
pub fn normalize_lookup_key(word: &str, lang: &str) -> String {
    let mut out = String::with_capacity(word.len());
    for ch in word.trim().chars() {
        if ch == '\u{0301}' {
            continue;
        }
        for lc in ch.to_lowercase() {
            if lang == "ru" && (lc == 'ё') {
                out.push('е');
            } else {
                out.push(lc);
            }
        }
    }
    out
}

/// Generates fallback candidate lemma keys for Russian and English inflection patterns.
pub fn morphological_candidates(norm: &str, lang: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut push_unique = |cand: String| {
        if cand != norm && cand.chars().count() >= 2 && !out.contains(&cand) {
            out.push(cand);
        }
    };

    if lang == "ru" {
        let ru_verb_map: &[(&str, &[&str])] = &[
            ("авшегося", &["аться", "ать"]),
            ("явшегося", &["яться", "ять"]),
            ("ившегося", &["иться", "ить"]),
            ("ующегося", &["оваться", "еваться"]),
            ("ющегося", &["ваться", "еваться"]),
            ("вшегося", &["ться", "ть"]),
            ("вшего", &["ть", "ать", "ить", "еть"]),
            ("вшая", &["ть", "ать", "ить", "еть"]),
            ("вшее", &["ть", "ать", "ить", "еть"]),
            ("вшие", &["ть", "ать", "ить", "еть"]),
            ("ться", &["ть"]),
            ("тся", &["ть", "ться"]),
            ("ала", &["ать"]),
            ("али", &["ать"]),
            ("ал", &["ать"]),
            ("ила", &["ить"]),
            ("или", &["ить"]),
            ("ил", &["ить"]),
            ("ела", &["еть"]),
            ("ели", &["еть"]),
            ("ел", &["еть"]),
            ("ут", &["ать", "уть", "ь"]),
            ("ют", &["ать", "ять", "еть"]),
            ("ит", &["ить", "еть"]),
            ("ат", &["ать", "ить"]),
            ("ят", &["ять", "ить"]),
            ("ет", &["ать", "еть", "ять", "уть"]),
        ];
        for (sfx, repls) in ru_verb_map {
            if let Some(stem) = norm.strip_suffix(sfx)
                && stem.chars().count() >= 2
            {
                for r in *repls {
                    push_unique(format!("{stem}{r}"));
                }
            }
        }

        let ru_adj_sfx = [
            "ого", "его", "ому", "ему", "ыми", "ими", "ою", "ею", "ая", "яя", "ое", "ее", "ые",
            "ие", "ую", "юю", "ом", "ем", "ым", "им", "ых", "их",
        ];
        for sfx in ru_adj_sfx {
            if let Some(stem) = norm.strip_suffix(sfx)
                && stem.chars().count() >= 2
            {
                push_unique(format!("{stem}ый"));
                push_unique(format!("{stem}ий"));
                push_unique(format!("{stem}ой"));
            }
        }

        let ru_noun_sfx = [
            "ами", "ями", "ах", "ях", "ов", "ев", "ей", "ом", "ем", "ой", "ам", "ям", "у", "ю",
            "а", "я", "ы", "и", "е",
        ];
        for sfx in ru_noun_sfx {
            if let Some(stem) = norm.strip_suffix(sfx)
                && stem.chars().count() >= 2
            {
                push_unique(stem.to_string());
                for end in ["а", "я", "о", "е", "ь", "й"] {
                    push_unique(format!("{stem}{end}"));
                }
            }
        }
        return out;
    }

    if let Some(stem) = norm.strip_suffix("ies") {
        push_unique(format!("{stem}y"));
    }
    if let Some(stem) = norm.strip_suffix("ied") {
        push_unique(format!("{stem}y"));
    }
    if let Some(stem) = norm.strip_suffix("ing") {
        push_unique(stem.to_string());
        push_unique(format!("{stem}e"));
        if let Some(ch) = stem.chars().last()
            && stem.ends_with(&format!("{ch}{ch}"))
            && stem.len() > 2
        {
            push_unique(stem[..stem.len() - ch.len_utf8()].to_string());
        }
    }
    if let Some(stem) = norm.strip_suffix("ed") {
        push_unique(stem.to_string());
        push_unique(format!("{stem}e"));
        if let Some(ch) = stem.chars().last()
            && stem.ends_with(&format!("{ch}{ch}"))
            && stem.len() > 2
        {
            push_unique(stem[..stem.len() - ch.len_utf8()].to_string());
        }
    }
    if let Some(stem) = norm.strip_suffix("es") {
        push_unique(stem.to_string());
        push_unique(format!("{stem}e"));
    }
    if let Some(stem) = norm.strip_suffix('s') {
        push_unique(stem.to_string());
    }
    if let Some(stem) = norm.strip_suffix("ly") {
        push_unique(stem.to_string());
    }
    if let Some(stem) = norm.strip_suffix("er") {
        push_unique(stem.to_string());
        push_unique(format!("{stem}e"));
    }
    if let Some(stem) = norm.strip_suffix("est") {
        push_unique(stem.to_string());
        push_unique(format!("{stem}e"));
    }
    out
}

pub struct DictionaryStore {
    dict_dir: PathBuf,
    offline_conn: Option<Connection>,
    cache_conn: Option<Connection>,
    lru_map: HashMap<(String, String), Vec<DictionaryEntry>>,
    lru_order: VecDeque<(String, String)>,
}

impl DictionaryStore {
    pub fn open(dict_dir: PathBuf) -> Self {
        let mut store = Self {
            dict_dir,
            offline_conn: None,
            cache_conn: None,
            lru_map: HashMap::new(),
            lru_order: VecDeque::new(),
        };
        store.reload();
        store
    }

    pub fn reload(&mut self) {
        self.lru_map.clear();
        self.lru_order.clear();
        let offline_path = self.dict_dir.join(OFFLINE_DB_FILENAME);
        self.offline_conn = if offline_path.is_file() {
            open_readonly_pack(&offline_path).ok()
        } else {
            None
        };
        self.cache_conn = open_cache_db(&self.dict_dir.join(CACHE_DB_FILENAME)).ok();
    }

    /// Copies a synced offline pack (e.g. from Syncthing's `Dictionaries/en-ru.db`) into local storage if newer.
    pub fn maybe_import_from(source_path: &Path, dict_dir: &Path) -> bool {
        let Ok(src_meta) = fs::metadata(source_path) else {
            return false;
        };
        if !source_path.is_file() || src_meta.len() < 4096 {
            return false;
        }
        let dest = dict_dir.join(OFFLINE_DB_FILENAME);
        if let Ok(dst_meta) = fs::metadata(&dest)
            && dst_meta.len() == src_meta.len()
            && dst_meta.modified().ok() >= src_meta.modified().ok()
        {
            return false;
        }
        if fs::create_dir_all(dict_dir).is_err() {
            return false;
        }
        let tmp = dict_dir.join(format!("{OFFLINE_DB_FILENAME}.import-tmp"));
        if fs::copy(source_path, &tmp).is_ok()
            && verify_pack_sqlite(&tmp).is_ok()
            && fs::rename(&tmp, &dest).is_ok()
        {
            return true;
        }
        let _ = fs::remove_file(&tmp);
        false
    }

    pub fn stats(&self) -> DictionaryStats {
        let offline_path = self.dict_dir.join(OFFLINE_DB_FILENAME);
        let size_bytes = fs::metadata(&offline_path).map(|m| m.len()).unwrap_or(0);
        let (installed, lemma_count, form_count) = if let Some(conn) = &self.offline_conn {
            let lemmas = conn
                .query_row("SELECT COUNT(*) FROM lemmas", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0)
                .max(0) as usize;
            let forms = conn
                .query_row("SELECT COUNT(*) FROM forms", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0)
                .max(0) as usize;
            (true, lemmas, forms)
        } else {
            (false, 0, 0)
        };
        let cached_count = if let Some(conn) = &self.cache_conn {
            conn.query_row("SELECT COUNT(*) FROM cache", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0)
                .max(0) as usize
        } else {
            0
        };
        DictionaryStats {
            installed,
            lemma_count,
            form_count,
            cached_count,
            size_bytes,
        }
    }

    fn insert_lru(&mut self, key: (String, String), entries: Vec<DictionaryEntry>) {
        if self.lru_map.contains_key(&key) {
            self.lru_map.insert(key, entries);
            return;
        }
        while self.lru_order.len() >= LRU_CAPACITY {
            if let Some(evict) = self.lru_order.pop_front() {
                self.lru_map.remove(&evict);
            }
        }
        self.lru_order.push_back(key.clone());
        self.lru_map.insert(key, entries);
    }

    /// Performs a synchronous sub-millisecond lookup across memory LRU, offline SQLite pack, and cache.
    pub fn lookup_offline(&mut self, raw: &str) -> Option<Vec<DictionaryEntry>> {
        let clean = extract_lookup_word(raw)?;
        let lang = detect_lang(&clean);
        let norm = normalize_lookup_key(&clean, lang);
        if norm.is_empty() {
            return None;
        }
        let cache_key = (lang.to_string(), norm.clone());
        if let Some(hit) = self.lru_map.get(&cache_key)
            && !hit.is_empty()
        {
            return Some(hit.clone());
        }

        if let Some(conn) = &self.offline_conn {
            let mut found = query_lemmas_exact(conn, lang, &norm, None, None);
            if found.is_empty() {
                found = query_forms_joined(conn, lang, &norm, &clean);
            }
            if found.is_empty() {
                for cand in morphological_candidates(&norm, lang) {
                    let hits = query_lemmas_exact(
                        conn,
                        lang,
                        &cand,
                        Some(clean.clone()),
                        Some("stem match".to_string()),
                    );
                    if !hits.is_empty() {
                        found = hits;
                        break;
                    }
                }
            }
            if !found.is_empty() {
                self.insert_lru(cache_key, found.clone());
                return Some(found);
            }
        }

        if let Some(conn) = &self.cache_conn
            && let Some(mut cached) = read_cached_entries(conn, lang, &norm)
        {
            for e in &mut cached {
                e.source = LookupSource::CachedOnline;
            }
            if !cached.is_empty() {
                self.insert_lru(cache_key, cached.clone());
                return Some(cached);
            }
        }

        None
    }
}

fn open_readonly_pack(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let _ = conn.execute_batch("PRAGMA query_only = ON; PRAGMA mmap_size = 67108864;");
    Ok(conn)
}

fn verify_pack_sqlite(path: &Path) -> Result<()> {
    let conn = open_readonly_pack(path)?;
    let _count: i64 = conn.query_row("SELECT COUNT(*) FROM lemmas", [], |r| r.get(0))?;
    Ok(())
}

fn open_cache_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cache (
            lang TEXT NOT NULL,
            word TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (lang, word)
        ) WITHOUT ROWID;",
    )?;
    Ok(conn)
}

fn row_to_entry(
    row: &rusqlite::Row<'_>,
    matched_form: Option<String>,
    gram_tag: Option<String>,
) -> rusqlite::Result<DictionaryEntry> {
    let lang: String = row.get(0)?;
    let headword: String = row.get(1)?;
    let stressed: String = row.get(2)?;
    let ipa: String = row.get(3)?;
    let pos: String = row.get(4)?;
    let glosses_json: String = row.get(5)?;
    let ru_translation: Option<String> = row.get(6)?;
    let glosses: Vec<String> = serde_json::from_str(&glosses_json).unwrap_or_default();
    Ok(DictionaryEntry {
        lang,
        headword,
        stressed,
        ipa,
        pos,
        glosses,
        ru_translation,
        matched_form,
        gram_tag,
        source: LookupSource::OfflinePack,
    })
}

fn query_lemmas_exact(
    conn: &Connection,
    lang: &str,
    word: &str,
    matched_form: Option<String>,
    gram_tag: Option<String>,
) -> Vec<DictionaryEntry> {
    let Ok(mut stmt) = conn.prepare_cached(
        "SELECT lang, word, stressed, ipa, pos, glosses_json, ru_tr
         FROM lemmas
         WHERE lang = ?1 AND word = ?2
         LIMIT 6",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map(params![lang, word], |row| {
        row_to_entry(row, matched_form.clone(), gram_tag.clone())
    }) else {
        return Vec::new();
    };
    rows.flatten().collect()
}

fn query_forms_joined(
    conn: &Connection,
    lang: &str,
    norm_form: &str,
    raw_form: &str,
) -> Vec<DictionaryEntry> {
    let Ok(mut stmt) = conn.prepare_cached(
        "SELECT l.lang, l.word, l.stressed, l.ipa, l.pos, l.glosses_json, l.ru_tr, f.gram
         FROM forms f
         JOIN lemmas l ON l.id = f.lemma_id
         WHERE f.lang = ?1 AND f.form = ?2
         LIMIT 6",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map(params![lang, norm_form], |row| {
        let gram: String = row.get(7)?;
        let gram_opt = (!gram.trim().is_empty()).then_some(gram);
        row_to_entry(row, Some(raw_form.to_string()), gram_opt)
    }) else {
        return Vec::new();
    };
    rows.flatten().collect()
}

fn read_cached_entries(conn: &Connection, lang: &str, norm: &str) -> Option<Vec<DictionaryEntry>> {
    let payload: String = conn
        .query_row(
            "SELECT payload_json FROM cache WHERE lang = ?1 AND word = ?2",
            params![lang, norm],
            |r| r.get(0),
        )
        .ok()?;
    serde_json::from_str(&payload).ok()
}

fn write_cached_entries(dict_dir: &Path, lang: &str, norm: &str, entries: &[DictionaryEntry]) {
    let Ok(conn) = open_cache_db(&dict_dir.join(CACHE_DB_FILENAME)) else {
        return;
    };
    let Ok(json) = serde_json::to_string(entries) else {
        return;
    };
    let ts = chrono::Utc::now().timestamp();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO cache (lang, word, payload_json, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![lang, norm, json, ts],
    );
}

fn build_dict_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(7)))
        .build()
        .into()
}

fn url_encode_segment(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

/// Fetches a Wiktionary or FreeDictionaryAPI entry over HTTPS, caches it in `user-cache.db`, and returns it.
pub fn lookup_online_and_cache(raw: &str, dict_dir: &Path) -> Option<Vec<DictionaryEntry>> {
    let clean = extract_lookup_word(raw)?;
    let lang = detect_lang(&clean);
    let norm = normalize_lookup_key(&clean, lang);
    if norm.is_empty() {
        return None;
    }

    let agent = build_dict_agent();
    let mut entries = fetch_wiktionary_entries(&agent, &norm, lang, 1);
    if entries.is_empty() && clean != norm {
        entries = fetch_wiktionary_entries(&agent, &clean, lang, 1);
    }
    if entries.is_empty() && lang == "en" {
        entries = fetch_freedict_en(&agent, &norm);
    }

    if entries.is_empty() {
        return None;
    }
    write_cached_entries(dict_dir, lang, &norm, &entries);
    Some(entries)
}

fn fetch_wiktionary_entries(
    agent: &ureq::Agent,
    term: &str,
    lang: &str,
    depth: u8,
) -> Vec<DictionaryEntry> {
    let url = format!(
        "https://en.wiktionary.org/api/rest_v1/page/definition/{}",
        url_encode_segment(term)
    );
    let Ok(mut resp) = agent
        .get(&url)
        .header("User-Agent", DICT_USER_AGENT)
        .call()
    else {
        return Vec::new();
    };
    let Ok(body) = resp.body_mut().read_to_string() else {
        return Vec::new();
    };
    let Ok(root) = serde_json::from_str::<Value>(&body) else {
        return Vec::new();
    };
    let Some(list) = root.get(lang).and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut form_targets = Vec::new();

    for section in list.iter().take(4) {
        let pos = section
            .get("partOfSpeech")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let Some(defs) = section.get("definitions").and_then(Value::as_array) else {
            continue;
        };
        let mut glosses = Vec::new();
        for d in defs.iter().take(4) {
            let Some(html) = d.get("definition").and_then(Value::as_str) else {
                continue;
            };
            let plain = strip_html_and_entities(html);
            if plain.is_empty() {
                continue;
            }
            if depth > 0
                && let Some(target) = extract_wiktionary_form_target(html, &plain)
                && !form_targets.contains(&target)
            {
                form_targets.push(target);
            }
            glosses.push(plain);
        }
        if !glosses.is_empty() {
            out.push(DictionaryEntry {
                lang: lang.to_string(),
                headword: term.to_string(),
                stressed: String::new(),
                ipa: String::new(),
                pos,
                glosses,
                ru_translation: None,
                matched_form: None,
                gram_tag: None,
                source: LookupSource::Online,
            });
        }
    }

    if depth > 0
        && let Some(lemma) = form_targets.into_iter().next()
        && normalize_lookup_key(&lemma, lang) != normalize_lookup_key(term, lang)
    {
        let mut lemma_entries = fetch_wiktionary_entries(agent, &lemma, lang, 0);
        if !lemma_entries.is_empty() {
            let form_note = out
                .first()
                .and_then(|e| e.glosses.first())
                .cloned();
            for le in &mut lemma_entries {
                le.matched_form = Some(term.to_string());
                if le.gram_tag.is_none() {
                    le.gram_tag = form_note.clone();
                }
            }
            return lemma_entries;
        }
    }

    out
}

fn extract_wiktionary_form_target(html: &str, plain: &str) -> Option<String> {
    let marker = "href=\"/wiki/";
    let lower_plain = plain.to_ascii_lowercase();
    let looks_inflection = lower_plain.contains(" of ")
        || lower_plain.contains("plural")
        || lower_plain.contains("genitive")
        || lower_plain.contains("dative")
        || lower_plain.contains("accusative")
        || lower_plain.contains("participle")
        || lower_plain.contains("tense")
        || lower_plain.contains("inflection");
    if !looks_inflection {
        return None;
    }
    let idx = html.rfind(marker)?;
    let tail = &html[idx + marker.len()..];
    let end = tail.find('"')?;
    let slug = tail[..end].split('#').next().unwrap_or("").trim();
    if slug.is_empty() || slug.contains(':') {
        return None;
    }
    let decoded = percent_encoding::percent_decode_str(slug)
        .decode_utf8()
        .ok()?
        .replace('_', " ");
    extract_lookup_word(&decoded)
}

fn fetch_freedict_en(agent: &ureq::Agent, term: &str) -> Vec<DictionaryEntry> {
    let url = format!(
        "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
        url_encode_segment(term)
    );
    let Ok(mut resp) = agent
        .get(&url)
        .header("User-Agent", DICT_USER_AGENT)
        .call()
    else {
        return Vec::new();
    };
    let Ok(body) = resp.body_mut().read_to_string() else {
        return Vec::new();
    };
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&body) else {
        return Vec::new();
    };
    let Some(first) = items.first() else {
        return Vec::new();
    };
    let headword = first
        .get("word")
        .and_then(Value::as_str)
        .unwrap_or(term)
        .trim()
        .to_string();
    let ipa = first
        .get("phonetic")
        .and_then(Value::as_str)
        .map(|s| s.trim().trim_matches('/').to_string())
        .or_else(|| {
            first
                .get("phonetics")
                .and_then(Value::as_array)
                .and_then(|arr| {
                    arr.iter()
                        .find_map(|p| p.get("text").and_then(Value::as_str))
                })
                .map(|s| s.trim().trim_matches('/').to_string())
        })
        .unwrap_or_default();

    let Some(meanings) = first.get("meanings").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for m in meanings.iter().take(3) {
        let pos = m
            .get("partOfSpeech")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let Some(defs) = m.get("definitions").and_then(Value::as_array) else {
            continue;
        };
        let glosses: Vec<String> = defs
            .iter()
            .take(3)
            .filter_map(|d| d.get("definition").and_then(Value::as_str))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !glosses.is_empty() {
            out.push(DictionaryEntry {
                lang: "en".to_string(),
                headword: headword.clone(),
                stressed: String::new(),
                ipa: ipa.clone(),
                pos,
                glosses,
                ru_translation: None,
                matched_form: None,
                gram_tag: None,
                source: LookupSource::Online,
            });
        }
    }
    out
}

/// Downloads and decompresses a gzipped offline SQLite dictionary pack atomically into `dict_dir/en-ru.db`.
pub fn download_offline_pack(dict_dir: &Path, url: &str) -> Result<DictionaryStats> {
    fs::create_dir_all(dict_dir)
        .with_context(|| format!("Failed to create dictionary directory {}", dict_dir.display()))?;

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(180)))
        .build()
        .into();
    let resp = agent
        .get(url)
        .header("User-Agent", DICT_USER_AGENT)
        .call()
        .with_context(|| format!("Failed to download dictionary pack from {url}"))?;

    let tmp_path = dict_dir.join(format!("{OFFLINE_DB_FILENAME}.download-tmp"));
    let mut decoder = GzDecoder::new(resp.into_body().into_reader());
    {
        let mut out_file = fs::File::create(&tmp_path)
            .with_context(|| format!("Failed to create temp file {}", tmp_path.display()))?;
        if let Err(err) = io::copy(&mut decoder, &mut out_file) {
            let _ = fs::remove_file(&tmp_path);
            return Err(err).context("Failed to decompress offline dictionary pack");
        }
    }

    if let Err(err) = verify_pack_sqlite(&tmp_path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err).context("Downloaded dictionary pack failed integrity check");
    }

    let final_path = dict_dir.join(OFFLINE_DB_FILENAME);
    fs::rename(&tmp_path, &final_path).with_context(|| {
        format!(
            "Failed to install dictionary pack at {}",
            final_path.display()
        )
    })?;

    Ok(DictionaryStore::open(dict_dir.to_path_buf()).stats())
}

#[cfg(test)]
mod tests {
    use super::{
        DictionaryEntry, DictionaryStore, LookupSource, OFFLINE_DB_FILENAME, detect_lang,
        extract_lookup_word, morphological_candidates, normalize_lookup_key,
    };
    use rusqlite::{Connection, params};
    use tempfile::tempdir;

    #[test]
    fn normalization_and_word_extraction_handle_russian_stress_and_punctuation() {
        assert_eq!(extract_lookup_word(" «Epiphany!» ").as_deref(), Some("Epiphany"));
        assert_eq!(
            extract_lookup_word("«переосмысливавшегося»,").as_deref(),
            Some("переосмысливавшегося")
        );
        assert_eq!(extract_lookup_word("this has three words"), None);
        assert_eq!(extract_lookup_word("..."), None);

        assert_eq!(detect_lang("переосмысливавшегося"), "ru");
        assert_eq!(detect_lang("epiphany"), "en");
        assert_eq!(normalize_lookup_key("Ёжик", "ru"), "ежик");
        assert_eq!(
            normalize_lookup_key("переосмы\u{0301}сливать", "ru"),
            "переосмысливать"
        );
    }

    #[test]
    fn morphological_candidates_cover_russian_and_english_inflections() {
        let ru_cands = morphological_candidates("переосмысливавшегося", "ru");
        assert!(ru_cands.iter().any(|c| c == "переосмысливаться"));
        let en_cands = morphological_candidates("cherries", "en");
        assert!(en_cands.iter().any(|c| c == "cherry"));
    }

    #[test]
    fn offline_pack_resolves_russian_inflections_with_stress_and_formats_note() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join(OFFLINE_DB_FILENAME);
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE lemmas (
                    id INTEGER PRIMARY KEY,
                    lang TEXT NOT NULL,
                    word TEXT NOT NULL,
                    stressed TEXT NOT NULL DEFAULT '',
                    ipa TEXT NOT NULL DEFAULT '',
                    pos TEXT NOT NULL DEFAULT '',
                    glosses_json TEXT NOT NULL,
                    ru_tr TEXT
                );
                CREATE INDEX idx_lemmas_lang_word ON lemmas(lang, word);
                CREATE TABLE forms (
                    form TEXT NOT NULL,
                    lang TEXT NOT NULL,
                    lemma_id INTEGER NOT NULL,
                    gram TEXT NOT NULL DEFAULT '',
                    PRIMARY KEY (lang, form, lemma_id)
                ) WITHOUT ROWID;",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO lemmas (id, lang, word, stressed, ipa, pos, glosses_json, ru_tr)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    1i64,
                    "ru",
                    "переосмысливаться",
                    "переосмы́сливаться",
                    "pʲɪrʲɪɐˈsmɨslʲɪvət͡sə",
                    "verb",
                    r#"["to be rethought","to be reconsidered"]"#,
                    Option::<String>::None
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO forms (form, lang, lemma_id, gram) VALUES (?1, ?2, ?3, ?4)",
                params!["переосмысливавшегося", "ru", 1i64, "past active participle gen. sg."],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO lemmas (id, lang, word, stressed, ipa, pos, glosses_json, ru_tr)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    2i64,
                    "en",
                    "epiphany",
                    "",
                    "ɪˈpɪfəni",
                    "noun",
                    r#"["a sudden intuitive realization"]"#,
                    Some("озарение, прозрение")
                ],
            )
            .unwrap();
        }

        let mut store = DictionaryStore::open(dir.path().to_path_buf());
        let stats = store.stats();
        assert!(stats.installed);
        assert_eq!(stats.lemma_count, 2);
        assert_eq!(stats.form_count, 1);

        let ru_hits = store.lookup_offline("«переосмысливавшегося»").unwrap();
        assert_eq!(ru_hits[0].display_headword(), "переосмы́сливаться");
        assert_eq!(
            ru_hits[0].gram_tag.as_deref(),
            Some("past active participle gen. sg.")
        );
        assert_eq!(ru_hits[0].source, LookupSource::OfflinePack);

        let en_hits = store.lookup_offline("epiphany").unwrap();
        let note = en_hits[0].format_as_note();
        assert!(note.contains("epiphany [ɪˈpɪfəni] (noun)"));
        assert!(note.contains("RU: озарение, прозрение"));

        let empty_entry = DictionaryEntry {
            lang: "en".into(),
            headword: "test".into(),
            stressed: String::new(),
            ipa: String::new(),
            pos: "noun".into(),
            glosses: vec!["trial".into()],
            ru_translation: None,
            matched_form: None,
            gram_tag: None,
            source: LookupSource::Online,
        };
        assert_eq!(empty_entry.format_as_note(), "test (noun) — trial");
    }
}
