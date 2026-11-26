//! Offline multilingual EN/RU/ZH-Hans SQLite dictionaries with morphology & Wiktionary fallback.

use crate::strip_html_and_entities;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use crate::library::{DictDefLang, is_cjk_char};

pub const DEFAULT_DICT_PACK_URL: &str =
    "https://github.com/vovakaravov/Bookworm/releases/download/dict-v2/bookworm-dict-def-en.db.gz";
pub const OFFLINE_DB_FILENAME: &str = "def-en.db";
pub const LEGACY_OFFLINE_DB_FILENAME: &str = "en-ru.db";
pub const CACHE_DB_FILENAME: &str = "user-cache.db";

const DICT_USER_AGENT: &str = "Bookworm/0.3 (multilingual-reader-dictionary)";
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
    #[serde(default)]
    pub def_lang: DictDefLang,
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

fn trim_lookup_punctuation(s: &str) -> &str {
    s.trim().trim_matches(|c: char| {
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
                    | '。'
                    | '，'
                    | '、'
                    | '；'
                    | '：'
                    | '！'
                    | '？'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
                    | '《'
                    | '》'
                    | '【'
                    | '】'
                    | '（'
                    | '）'
                    | '·'
                    | '～'
            )
    })
}

/// Extracts a clean lookup term from a reader selection (1–2 non-CJK words or ≤ 8 CJK chars).
pub fn extract_lookup_word(selection: &str) -> Option<String> {
    let trimmed = trim_lookup_punctuation(selection).trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().any(is_cjk_char) {
        let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        let cc = compact.chars().count();
        if cc == 0 || cc > 8 || !compact.chars().any(is_cjk_char) {
            return None;
        }
        return Some(compact);
    }
    if trimmed.chars().count() > 48 {
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
    if word.chars().any(is_cjk_char) {
        "zh"
    } else if word.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)) {
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
        if lang == "zh" {
            if !ch.is_whitespace() {
                out.push(ch);
            }
            continue;
        }
        for lc in ch.to_lowercase() {
            if lang == "ru" && lc == 'ё' {
                out.push('е');
            } else {
                out.push(lc);
            }
        }
    }
    out
}

fn cache_lang_key(def_lang: DictDefLang, word_lang: &str) -> String {
    format!("{}:{word_lang}", def_lang.code())
}

/// Generates fallback candidate lemma keys for Russian and English inflection patterns.
pub fn morphological_candidates(norm: &str, lang: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut push_unique = |cand: String| {
        if cand != norm && cand.chars().count() >= 2 && !out.contains(&cand) {
            out.push(cand);
        }
    };

    if lang == "zh" {
        return out;
    }

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
    offline_conns: HashMap<DictDefLang, Connection>,
    offline_paths: HashMap<DictDefLang, PathBuf>,
    cache_conn: Option<Connection>,
    lru_map: HashMap<(DictDefLang, String, String), Vec<DictionaryEntry>>,
    lru_order: VecDeque<(DictDefLang, String, String)>,
}

impl DictionaryStore {
    pub fn open(dict_dir: PathBuf) -> Self {
        let mut store = Self {
            dict_dir,
            offline_conns: HashMap::new(),
            offline_paths: HashMap::new(),
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
        self.offline_conns.clear();
        self.offline_paths.clear();

        for &dl in DictDefLang::all() {
            let primary = self.dict_dir.join(dl.pack_filename());
            let chosen = if primary.is_file() {
                Some(primary)
            } else if dl == DictDefLang::En {
                let legacy = self.dict_dir.join(LEGACY_OFFLINE_DB_FILENAME);
                legacy.is_file().then_some(legacy)
            } else {
                None
            };
            if let Some(path) = chosen
                && let Ok(conn) = open_readonly_pack(&path)
            {
                self.offline_conns.insert(dl, conn);
                self.offline_paths.insert(dl, path);
            }
        }
        self.cache_conn = open_cache_db(&self.dict_dir.join(CACHE_DB_FILENAME)).ok();
    }

    fn import_single_pack(source_path: &Path, dest: &Path, dict_dir: &Path) -> bool {
        let Ok(src_meta) = fs::metadata(source_path) else {
            return false;
        };
        if !source_path.is_file() || src_meta.len() < 4096 {
            return false;
        }
        if let Ok(dst_meta) = fs::metadata(dest)
            && dst_meta.len() == src_meta.len()
            && dst_meta.modified().ok() >= src_meta.modified().ok()
        {
            return false;
        }
        if fs::create_dir_all(dict_dir).is_err() {
            return false;
        }
        let tmp = dict_dir.join(format!(
            "{}.import-tmp",
            dest.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("pack.db")
        ));
        if fs::copy(source_path, &tmp).is_ok()
            && verify_pack_sqlite(&tmp).is_ok()
            && fs::rename(&tmp, dest).is_ok()
        {
            return true;
        }
        let _ = fs::remove_file(&tmp);
        false
    }

    /// Copies a synced offline pack (e.g. from Syncthing's `Dictionaries/en-ru.db`) into local storage.
    pub fn maybe_import_from(source_path: &Path, dict_dir: &Path) -> bool {
        Self::import_single_pack(source_path, &dict_dir.join(OFFLINE_DB_FILENAME), dict_dir)
    }

    /// Imports any synced offline packs (`def-en.db`, `def-ru.db`, `def-zh.db`, or legacy `en-ru.db`).
    pub fn maybe_import_from_dir(source_dir: &Path, dict_dir: &Path) -> bool {
        let mut any = false;
        for &dl in DictDefLang::all() {
            let src = source_dir.join(dl.pack_filename());
            let dst = dict_dir.join(dl.pack_filename());
            any |= Self::import_single_pack(&src, &dst, dict_dir);
        }
        let legacy_src = source_dir.join(LEGACY_OFFLINE_DB_FILENAME);
        let en_dst = dict_dir.join(DictDefLang::En.pack_filename());
        if !en_dst.is_file() {
            any |= Self::import_single_pack(&legacy_src, &en_dst, dict_dir);
        }
        any
    }

    pub fn stats_for(&self, def_lang: DictDefLang) -> DictionaryStats {
        let size_bytes = self
            .offline_paths
            .get(&def_lang)
            .and_then(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .unwrap_or(0);
        let (installed, lemma_count, form_count) =
            if let Some(conn) = self.offline_conns.get(&def_lang) {
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
            let prefix = format!("{}:%", def_lang.code());
            conn.query_row(
                "SELECT COUNT(*) FROM cache WHERE lang LIKE ?1",
                params![prefix],
                |r| r.get::<_, i64>(0),
            )
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

    pub fn all_stats(&self) -> [(DictDefLang, DictionaryStats); 3] {
        [
            (DictDefLang::En, self.stats_for(DictDefLang::En)),
            (DictDefLang::Ru, self.stats_for(DictDefLang::Ru)),
            (DictDefLang::ZhHans, self.stats_for(DictDefLang::ZhHans)),
        ]
    }

    pub fn stats(&self) -> DictionaryStats {
        self.stats_for(DictDefLang::En)
    }

    /// Fast indexed check for whether a 2–4 char Mandarin compound exists in an installed pack.
    pub fn has_cjk_word(&self, word: &str, def_lang: DictDefLang) -> bool {
        let Some(conn) = self
            .offline_conns
            .get(&def_lang)
            .or_else(|| self.offline_conns.get(&DictDefLang::En))
        else {
            return false;
        };
        let in_lemmas = conn
            .prepare_cached("SELECT 1 FROM lemmas WHERE lang = 'zh' AND word = ?1 LIMIT 1")
            .ok()
            .and_then(|mut s| s.exists(params![word]).ok())
            .unwrap_or(false);
        if in_lemmas {
            return true;
        }
        conn.prepare_cached("SELECT 1 FROM forms WHERE lang = 'zh' AND form = ?1 LIMIT 1")
            .ok()
            .and_then(|mut s| s.exists(params![word]).ok())
            .unwrap_or(false)
    }

    fn insert_lru(
        &mut self,
        key: (DictDefLang, String, String),
        entries: Vec<DictionaryEntry>,
    ) {
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

    /// Synchronous sub-millisecond lookup across LRU, offline SQLite pack, and cached online results.
    pub fn lookup_offline(
        &mut self,
        raw: &str,
        def_lang: DictDefLang,
    ) -> Option<Vec<DictionaryEntry>> {
        let clean = extract_lookup_word(raw)?;
        let word_lang = detect_lang(&clean);
        let norm = normalize_lookup_key(&clean, word_lang);
        if norm.is_empty() {
            return None;
        }
        let cache_key = (def_lang, word_lang.to_string(), norm.clone());
        if let Some(hit) = self.lru_map.get(&cache_key)
            && !hit.is_empty()
        {
            return Some(hit.clone());
        }

        if let Some(conn) = self.offline_conns.get(&def_lang) {
            let mut found = query_lemmas_exact(conn, def_lang, word_lang, &norm, None, None);
            if found.is_empty() {
                found = query_forms_joined(conn, def_lang, word_lang, &norm, &clean);
            }
            if found.is_empty() {
                for cand in morphological_candidates(&norm, word_lang) {
                    let hits = query_lemmas_exact(
                        conn,
                        def_lang,
                        word_lang,
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
            && let Some(mut cached) =
                read_cached_entries(conn, &cache_lang_key(def_lang, word_lang), &norm)
        {
            for e in &mut cached {
                e.def_lang = def_lang;
                e.source = LookupSource::CachedOnline;
            }
            if !cached.is_empty() {
                self.insert_lru(cache_key, cached.clone());
                return Some(cached);
            }
        }

        None
    }

    /// Offline fallback to the English-definition pack if another definition language misses offline.
    pub fn lookup_offline_fallback_en(&mut self, raw: &str) -> Option<Vec<DictionaryEntry>> {
        self.lookup_offline(raw, DictDefLang::En)
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
    def_lang: DictDefLang,
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
        def_lang,
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
    def_lang: DictDefLang,
    word_lang: &str,
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
    let Ok(rows) = stmt.query_map(params![word_lang, word], |row| {
        row_to_entry(row, def_lang, matched_form.clone(), gram_tag.clone())
    }) else {
        return Vec::new();
    };
    rows.flatten().collect()
}

fn query_forms_joined(
    conn: &Connection,
    def_lang: DictDefLang,
    word_lang: &str,
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
    let Ok(rows) = stmt.query_map(params![word_lang, norm_form], |row| {
        let gram: String = row.get(7)?;
        let gram_opt = (!gram.trim().is_empty()).then_some(gram);
        row_to_entry(row, def_lang, Some(raw_form.to_string()), gram_opt)
    }) else {
        return Vec::new();
    };
    rows.flatten().collect()
}

fn read_cached_entries(
    conn: &Connection,
    cache_lang: &str,
    norm: &str,
) -> Option<Vec<DictionaryEntry>> {
    let payload: String = conn
        .query_row(
            "SELECT payload_json FROM cache WHERE lang = ?1 AND word = ?2",
            params![cache_lang, norm],
            |r| r.get(0),
        )
        .ok()?;
    serde_json::from_str(&payload).ok()
}

fn write_cached_entries(
    dict_dir: &Path,
    def_lang: DictDefLang,
    word_lang: &str,
    norm: &str,
    entries: &[DictionaryEntry],
) {
    let Ok(conn) = open_cache_db(&dict_dir.join(CACHE_DB_FILENAME)) else {
        return;
    };
    let Ok(json) = serde_json::to_string(entries) else {
        return;
    };
    let ts = chrono::Utc::now().timestamp();
    let ck = cache_lang_key(def_lang, word_lang);
    let _ = conn.execute(
        "INSERT OR REPLACE INTO cache (lang, word, payload_json, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![ck, norm, json, ts],
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

/// Fetches a Wiktionary or FreeDictionaryAPI entry in the selected definition language and caches it.
pub fn lookup_online_and_cache(
    raw: &str,
    def_lang: DictDefLang,
    dict_dir: &Path,
) -> Option<Vec<DictionaryEntry>> {
    let clean = extract_lookup_word(raw)?;
    let word_lang = detect_lang(&clean);
    let norm = normalize_lookup_key(&clean, word_lang);
    if norm.is_empty() {
        return None;
    }

    let agent = build_dict_agent();
    let mut entries = match def_lang {
        DictDefLang::En => {
            let mut res = if word_lang != "zh" {
                fetch_wiktionary_rest_en(&agent, &norm, word_lang, 1)
            } else {
                Vec::new()
            };
            if res.is_empty() && word_lang != "zh" && clean != norm {
                res = fetch_wiktionary_rest_en(&agent, &clean, word_lang, 1);
            }
            if res.is_empty() {
                res = fetch_mediawiki_wiktionary(
                    &agent,
                    "en.wiktionary.org",
                    &norm,
                    word_lang,
                    DictDefLang::En,
                    false,
                    1,
                );
            }
            if res.is_empty() && word_lang == "en" {
                res = fetch_freedict_en(&agent, &norm);
            }
            res
        }
        DictDefLang::Ru => {
            let mut res = fetch_mediawiki_wiktionary(
                &agent,
                "ru.wiktionary.org",
                &norm,
                word_lang,
                DictDefLang::Ru,
                false,
                1,
            );
            if res.is_empty() && clean != norm {
                res = fetch_mediawiki_wiktionary(
                    &agent,
                    "ru.wiktionary.org",
                    &clean,
                    word_lang,
                    DictDefLang::Ru,
                    false,
                    1,
                );
            }
            res
        }
        DictDefLang::ZhHans => fetch_mediawiki_wiktionary(
            &agent,
            "zh.wiktionary.org",
            &norm,
            word_lang,
            DictDefLang::ZhHans,
            true,
            1,
        ),
    };

    for e in &mut entries {
        e.def_lang = def_lang;
    }

    if entries.is_empty() {
        return None;
    }
    write_cached_entries(dict_dir, def_lang, word_lang, &norm, &entries);
    Some(entries)
}

fn fetch_wiktionary_rest_en(
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
                def_lang: DictDefLang::En,
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
        let mut lemma_entries = fetch_wiktionary_rest_en(agent, &lemma, lang, 0);
        if !lemma_entries.is_empty() {
            let form_note = out.first().and_then(|e| e.glosses.first()).cloned();
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

fn section_heading_ids(def_lang: DictDefLang, word_lang: &str) -> &'static [&'static str] {
    match (def_lang, word_lang) {
        (DictDefLang::En, "en") => &["id=\"English\""],
        (DictDefLang::En, "ru") => &["id=\"Russian\""],
        (DictDefLang::En, "zh") => &["id=\"Chinese\"", "id=\"Mandarin\""],
        (DictDefLang::Ru, "en") => &["id=\"Английский\""],
        (DictDefLang::Ru, "ru") => &["id=\"Русский\""],
        (DictDefLang::Ru, "zh") => &["id=\"Китайский\""],
        (DictDefLang::ZhHans, "zh") => &["id=\"汉语\"", "id=\"漢語\"", "id=\"官话\"", "id=\"官話\""],
        (DictDefLang::ZhHans, "en") => &["id=\"英语\"", "id=\"英語\""],
        (DictDefLang::ZhHans, "ru") => &["id=\"俄语\"", "id=\"俄語\""],
        _ => &[],
    }
}

fn slice_language_section(html: &str, def_lang: DictDefLang, word_lang: &str) -> Option<String> {
    let ids = section_heading_ids(def_lang, word_lang);
    let mut start_ix: Option<usize> = None;
    for pat in ids {
        if let Some(pos) = html.find(pat) {
            start_ix = Some(pos);
            break;
        }
    }
    let start = start_ix?;
    let tail = &html[start..];
    // Skip past the closing `</h2>` of the matched language header, then stop at next `<h2`.
    let body_start = tail.find("</h2>").map(|i| i + 5).unwrap_or(0);
    let rest = &tail[body_start..];
    let end = rest
        .find("<div class=\"mw-heading mw-heading2\"")
        .or_else(|| rest.find("<h2"))
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn strip_nested_tag_blocks(mut html: String, open_prefix: &str, close_tag: &str) -> String {
    loop {
        let Some(start) = html.find(open_prefix) else {
            break;
        };
        let Some(rel_end) = html[start..].find(close_tag) else {
            html.truncate(start);
            break;
        };
        let end = start + rel_end + close_tag.len();
        html.replace_range(start..end, " ");
    }
    html
}

fn clean_mediawiki_item(raw_li: &str) -> String {
    let s = strip_nested_tag_blocks(raw_li.to_string(), "<dl", "</dl>");
    let s = strip_nested_tag_blocks(s, "<ul", "</ul>");
    let s = strip_nested_tag_blocks(s, "<span class=\"example-", "</span></span>");
    let plain = strip_html_and_entities(&s);
    let trimmed = plain.split('\n').next().unwrap_or("").trim().to_string();
    trimmed
}

fn extract_last_wiki_link_target(fragment: &str) -> Option<String> {
    let marker = "href=\"/wiki/";
    let mut search_from = 0usize;
    let mut last_hit: Option<String> = None;
    while let Some(rel) = fragment[search_from..].find(marker) {
        let abs = search_from + rel + marker.len();
        let Some(end) = fragment[abs..].find('"') else {
            break;
        };
        let slug = fragment[abs..abs + end].split('#').next().unwrap_or("").trim();
        search_from = abs + end;
        if slug.is_empty() || slug.contains(':') {
            continue;
        }
        if let Ok(decoded) = percent_decode_str(slug).decode_utf8()
            && let Some(clean) = extract_lookup_word(&decoded.replace('_', " "))
        {
            last_hit = Some(clean);
        }
    }
    last_hit
}

/// Parses MediaWiki `action=parse&prop=text` HTML for EN/RU/ZH Wiktionary definitions & redirects.
pub fn parse_mediawiki_definition_html(
    html: &str,
    term: &str,
    word_lang: &str,
    def_lang: DictDefLang,
) -> (Vec<DictionaryEntry>, Option<String>) {
    let section = slice_language_section(html, def_lang, word_lang).unwrap_or_default();
    if section.is_empty() {
        return (Vec::new(), None);
    }

    let mut glosses = Vec::new();
    let mut redirect_target: Option<String> = None;

    // Look for zh-see / simplified-from links (common on en.wiktionary for Simplified Chinese entries)
    if word_lang == "zh" {
        for marker in [
            "For pronunciation and definitions",
            "Simplified from",
            "simplified form of",
            "简体",
            "簡體",
        ] {
            if let Some(ix) = section.find(marker) {
                let win_end = crate::library::clamp_to_char_boundary(
                    &section,
                    (ix + 700).min(section.len()),
                );
                let window = &section[ix..win_end];
                let target_slice = window
                    .find("see ")
                    .map(|s| &window[s + 4..])
                    .unwrap_or(window);
                if let Some(t) = extract_last_wiki_link_target(target_slice)
                    .or_else(|| extract_last_wiki_link_target(window))
                    && t != term
                {
                    redirect_target = Some(t);
                    break;
                }
            }
        }
    }

    let mut pos_ix = 0usize;
    while let Some(ol_rel) = section[pos_ix..].find("<ol") {
        let ol_open = pos_ix + ol_rel;
        let Some(tag_close_rel) = section[ol_open..].find('>') else {
            break;
        };
        let body_start = ol_open + tag_close_rel + 1;
        let Some(ol_end_rel) = section[body_start..].find("</ol>") else {
            break;
        };
        let ol_body = &section[body_start..body_start + ol_end_rel];
        pos_ix = body_start + ol_end_rel + 5;

        let mut li_pos = 0usize;
        while let Some(li_rel) = ol_body[li_pos..].find("<li") {
            let li_open = li_pos + li_rel;
            let Some(gt_rel) = ol_body[li_open..].find('>') else {
                break;
            };
            let li_start = li_open + gt_rel + 1;
            let li_end = ol_body[li_start..]
                .find("</li>")
                .map(|e| li_start + e)
                .unwrap_or(ol_body.len());
            let raw_li = &ol_body[li_start..li_end];
            li_pos = (li_end + 5).min(ol_body.len());

            let plain = clean_mediawiki_item(raw_li);
            if plain.is_empty()
                || plain.contains("пока не переведено")
                || plain.contains("Это слово или выражение")
            {
                continue;
            }
            if redirect_target.is_none()
                && extract_wiktionary_form_target(raw_li, &plain).is_some()
            {
                redirect_target = extract_last_wiki_link_target(raw_li);
            }
            if !glosses.contains(&plain) {
                glosses.push(plain);
                if glosses.len() >= 4 {
                    break;
                }
            }
        }
        if glosses.len() >= 4 {
            break;
        }
    }

    if glosses.is_empty() {
        return (Vec::new(), redirect_target);
    }

    let entry = DictionaryEntry {
        lang: word_lang.to_string(),
        def_lang,
        headword: term.to_string(),
        stressed: String::new(),
        ipa: String::new(),
        pos: String::new(),
        glosses,
        ru_translation: None,
        matched_form: None,
        gram_tag: None,
        source: LookupSource::Online,
    };
    (vec![entry], redirect_target)
}

fn fetch_mediawiki_wiktionary(
    agent: &ureq::Agent,
    host: &str,
    term: &str,
    word_lang: &str,
    def_lang: DictDefLang,
    zh_cn_variant: bool,
    depth: u8,
) -> Vec<DictionaryEntry> {
    let variant_q = if zh_cn_variant { "&variant=zh-cn" } else { "" };
    let url = format!(
        "https://{host}/w/api.php?action=parse&page={}&prop=text&format=json&formatversion=2&redirects=1{variant_q}",
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
    let Some(html) = root
        .get("parse")
        .and_then(|p| p.get("text"))
        .and_then(Value::as_str)
    else {
        return Vec::new();
    };

    let (entries, redirect_target) =
        parse_mediawiki_definition_html(html, term, word_lang, def_lang);
    if depth > 0
        && let Some(target) = redirect_target
        && normalize_lookup_key(&target, word_lang) != normalize_lookup_key(term, word_lang)
        && (entries.is_empty() || word_lang == "zh")
    {
        let mut target_entries = fetch_mediawiki_wiktionary(
            agent,
            host,
            &target,
            word_lang,
            def_lang,
            zh_cn_variant,
            0,
        );
        if !target_entries.is_empty() {
            for te in &mut target_entries {
                te.matched_form = Some(term.to_string());
                if te.gram_tag.is_none() {
                    te.gram_tag = entries.first().and_then(|e| e.glosses.first()).cloned();
                }
            }
            return target_entries;
        }
    }
    entries
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
        || lower_plain.contains("inflection")
        || plain.contains("форма")
        || plain.contains("简体")
        || plain.contains("簡體");
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
    let decoded = percent_decode_str(slug).decode_utf8().ok()?.replace('_', " ");
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
                .and_then(|arr| arr.iter().find_map(|p| p.get("text").and_then(Value::as_str)))
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
                def_lang: DictDefLang::En,
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

/// Downloads and decompresses a gzipped offline SQLite dictionary pack into `dict_dir/def-en.db`.
pub fn download_offline_pack(dict_dir: &Path, url: &str) -> Result<DictionaryStats> {
    download_offline_pack_into(dict_dir, DictDefLang::En, url)
}

/// Downloads and installs the offline SQLite dictionary pack for the specified definition language.
pub fn download_offline_pack_for(dict_dir: &Path, def_lang: DictDefLang) -> Result<DictionaryStats> {
    download_offline_pack_into(dict_dir, def_lang, def_lang.pack_url())
}

fn download_offline_pack_into(
    dict_dir: &Path,
    def_lang: DictDefLang,
    url: &str,
) -> Result<DictionaryStats> {
    fs::create_dir_all(dict_dir)
        .with_context(|| format!("Failed to create dictionary directory {}", dict_dir.display()))?;

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(240)))
        .build()
        .into();
    let resp = agent
        .get(url)
        .header("User-Agent", DICT_USER_AGENT)
        .call()
        .with_context(|| format!("Failed to download dictionary pack from {url}"))?;

    let fname = def_lang.pack_filename();
    let tmp_path = dict_dir.join(format!("{fname}.download-tmp"));
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

    let final_path = dict_dir.join(fname);
    fs::rename(&tmp_path, &final_path).with_context(|| {
        format!(
            "Failed to install dictionary pack at {}",
            final_path.display()
        )
    })?;

    Ok(DictionaryStore::open(dict_dir.to_path_buf()).stats_for(def_lang))
}

#[cfg(test)]
mod tests {
    use super::{
        DictDefLang, DictionaryEntry, DictionaryStore, LookupSource, detect_lang,
        extract_lookup_word, morphological_candidates, normalize_lookup_key,
        parse_mediawiki_definition_html, write_cached_entries,
    };
    use rusqlite::{Connection, params};
    use tempfile::tempdir;

    #[test]
    fn normalization_and_word_extraction_handle_russian_chinese_and_punctuation() {
        assert_eq!(extract_lookup_word(" «Epiphany!» ").as_deref(), Some("Epiphany"));
        assert_eq!(
            extract_lookup_word("«переосмысливавшегося»,").as_deref(),
            Some("переосмысливавшегося")
        );
        assert_eq!(extract_lookup_word("《阅读》，").as_deref(), Some("阅读"));
        assert_eq!(extract_lookup_word("this has three words"), None);
        assert_eq!(extract_lookup_word("这是一段很长很长的句子"), None);
        assert_eq!(extract_lookup_word("..."), None);

        assert_eq!(detect_lang("переосмысливавшегося"), "ru");
        assert_eq!(detect_lang("阅读"), "zh");
        assert_eq!(detect_lang("epiphany"), "en");
        assert_eq!(normalize_lookup_key("Ёжик", "ru"), "ежик");
        assert_eq!(
            normalize_lookup_key("переосмы\u{0301}сливать", "ru"),
            "переосмысливать"
        );
        assert_eq!(normalize_lookup_key(" 阅 读 ", "zh"), "阅读");
    }

    #[test]
    fn morphological_candidates_cover_russian_and_english_inflections() {
        let ru_cands = morphological_candidates("переосмысливавшегося", "ru");
        assert!(ru_cands.iter().any(|c| c == "переосмысливаться"));
        let en_cands = morphological_candidates("cherries", "en");
        assert!(en_cands.iter().any(|c| c == "cherry"));
    }

    #[test]
    fn offline_pack_resolves_def_langs_and_cjk_compounds() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join(DictDefLang::En.pack_filename());
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
            conn.execute(
                "INSERT INTO lemmas (id, lang, word, stressed, ipa, pos, glosses_json, ru_tr)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    3i64,
                    "zh",
                    "阅读",
                    "",
                    "yuèdú",
                    "verb",
                    r#"["to read"]"#,
                    Option::<String>::None
                ],
            )
            .unwrap();
        }

        let mut store = DictionaryStore::open(dir.path().to_path_buf());
        let stats = store.stats();
        assert!(stats.installed);
        assert_eq!(stats.lemma_count, 3);
        assert_eq!(stats.form_count, 1);
        assert!(store.has_cjk_word("阅读", DictDefLang::En));
        assert!(!store.has_cjk_word("不存在", DictDefLang::En));

        let ru_hits = store
            .lookup_offline("«переосмысливавшегося»", DictDefLang::En)
            .unwrap();
        assert_eq!(ru_hits[0].display_headword(), "переосмы́сливаться");
        assert_eq!(
            ru_hits[0].gram_tag.as_deref(),
            Some("past active participle gen. sg.")
        );
        assert_eq!(ru_hits[0].source, LookupSource::OfflinePack);

        let zh_hits = store.lookup_offline("阅读", DictDefLang::En).unwrap();
        assert_eq!(zh_hits[0].ipa, "yuèdú");
        assert_eq!(zh_hits[0].glosses[0], "to read");

        // Cache entries are strictly isolated per DictDefLang
        let ru_def_entry = DictionaryEntry {
            lang: "en".into(),
            def_lang: DictDefLang::Ru,
            headword: "epiphany".into(),
            stressed: String::new(),
            ipa: String::new(),
            pos: "noun".into(),
            glosses: vec!["прозрение".into()],
            ru_translation: None,
            matched_form: None,
            gram_tag: None,
            source: LookupSource::Online,
        };
        write_cached_entries(dir.path(), DictDefLang::Ru, "en", "epiphany", &[ru_def_entry]);
        store.reload();

        let en_cached = store.lookup_offline("epiphany", DictDefLang::En).unwrap();
        assert_eq!(en_cached[0].def_lang, DictDefLang::En);
        assert_eq!(en_cached[0].glosses[0], "a sudden intuitive realization");

        let ru_cached = store.lookup_offline("epiphany", DictDefLang::Ru).unwrap();
        assert_eq!(ru_cached[0].def_lang, DictDefLang::Ru);
        assert_eq!(ru_cached[0].glosses[0], "прозрение");
        assert!(store.lookup_offline("epiphany", DictDefLang::ZhHans).is_none());
    }

    #[test]
    fn parse_mediawiki_definition_html_extracts_sections_and_redirects() {
        let ru_html = r#"
            <h2 id="Русский">Русский</h2>
            <ol>
                <li>носитель информации в виде сброшюрованных листов бумаги<dl><dd>Пример</dd></dl></li>
            </ol>
            <h2 id="Украинский">Украинский</h2>
            <ol><li>другое</li></ol>
        "#;
        let (entries, redir) =
            parse_mediawiki_definition_html(ru_html, "книга", "ru", DictDefLang::Ru);
        assert_eq!(redir, None);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].glosses[0],
            "носитель информации в виде сброшюрованных листов бумаги"
        );

        let zh_see_html = r#"
            <h2 id="Chinese">Chinese</h2>
            <table><tr><td><b>For pronunciation and definitions of </b><a href="/wiki/%E9%98%85">阅</a> – see <span class="Hant"><a href="/wiki/%E9%96%B1%E8%AE%80#Chinese">閱讀</a></span>.</td></tr></table>
        "#;
        let (zh_entries, zh_redir) =
            parse_mediawiki_definition_html(zh_see_html, "阅读", "zh", DictDefLang::En);
        assert!(zh_entries.is_empty());
        assert_eq!(zh_redir.as_deref(), Some("閱讀"));
    }
}
