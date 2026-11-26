#!/usr/bin/env python3
"""Build Bookworm's compressed offline EN/RU SQLite dictionary pack from Kaikki Wiktionary dumps."""

import gzip
import json
import os
import shutil
import sqlite3
import sys
import urllib.request
from pathlib import Path

BUILD_DIR = Path("/tmp/bookworm-dict-build")
RU_URL = "https://kaikki.org/dictionary/Russian/kaikki.org-dictionary-Russian.jsonl.gz"
EN_URL = "https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl.gz"

OUT_DB = BUILD_DIR / "bookworm-dict-en-ru.db"
OUT_GZ = BUILD_DIR / "bookworm-dict-en-ru.db.gz"

SKIP_FORM_TAGS = {
    "table-tags",
    "inflection-template",
    "canonical",
    "romanization",
    "class",
    "entry",
    "archaic",
}


def norm_key(word: str, lang: str) -> str:
    w = word.strip().replace("\u0301", "").lower()
    if lang == "ru":
        w = w.replace("ё", "е")
    return w


def download_if_missing(url: str, dest: Path) -> None:
    if dest.exists() and dest.stat().st_size > 1_000_000:
        print(f"Using cached {dest} ({dest.stat().st_size / 1e6:.1f} MB)")
        return
    tmp = dest.with_suffix(".tmp")
    print(f"Downloading {url} -> {dest} ...")
    req = urllib.request.Request(url, headers={"User-Agent": "Bookworm-Dict-Builder/1.0"})
    with urllib.request.urlopen(req, timeout=180) as resp, open(tmp, "wb") as f:
        shutil.copyfileobj(resp, f, length=1024 * 1024)
    tmp.replace(dest)
    print(f"Downloaded {dest.stat().st_size / 1e6:.1f} MB")


def extract_ipa(obj: dict) -> str:
    for snd in obj.get("sounds") or []:
        ipa = snd.get("ipa")
        if isinstance(ipa, str) and ipa.strip():
            return ipa.strip().strip("/")
    return ""


def extract_ru_stressed(obj: dict, headword: str) -> str:
    for f in obj.get("forms") or []:
        tags = set(f.get("tags") or [])
        if "canonical" in tags:
            val = (f.get("form") or "").strip()
            if "\u0301" in val:
                return val
    return ""


def extract_glosses(obj: dict) -> list[str]:
    out: list[str] = []
    seen: set[str] = set()
    for sense in obj.get("senses") or []:
        if len(out) >= 4:
            break
        raw_glosses = sense.get("glosses") or []
        if not raw_glosses:
            continue
        g = " — ".join(str(x).strip() for x in raw_glosses if str(x).strip())
        if not g:
            continue
        if len(g) > 180:
            g = g[:177].rstrip() + "..."
        low = g.lower()
        if low not in seen:
            seen.add(low)
            out.append(g)
    return out


def extract_ru_translations(obj: dict) -> str | None:
    trs: list[str] = []
    seen: set[str] = set()
    for t in obj.get("translations") or []:
        if t.get("lang_code") == "ru" or t.get("code") == "ru":
            w = (t.get("word") or "").strip()
            if w and w.lower() not in seen and " " not in w[:18]:
                seen.add(w.lower())
                trs.append(w)
                if len(trs) >= 4:
                    break
    if not trs:
        for s in obj.get("senses") or []:
            for t in s.get("translations") or []:
                if t.get("lang_code") == "ru" or t.get("code") == "ru":
                    w = (t.get("word") or "").strip()
                    if w and w.lower() not in seen:
                        seen.add(w.lower())
                        trs.append(w)
                        if len(trs) >= 4:
                            break
            if len(trs) >= 4:
                break
    return ", ".join(trs) if trs else None


def is_clean_headword(word: str) -> bool:
    if not word or len(word) > 48:
        return False
    if len(word.split()) > 2:
        return False
    return any(ch.isalpha() for ch in word)


def ingest_dump(conn: sqlite3.Connection, gz_path: Path, lang: str) -> tuple[int, int]:
    cur = conn.cursor()
    lemma_rows = []
    form_rows = []
    next_id_row = cur.execute("SELECT COALESCE(MAX(id), 0) FROM lemmas").fetchone()
    lemma_id = int(next_id_row[0])

    total_lemmas = 0
    total_forms = 0
    seen_forms: set[tuple[str, int]] = set()

    with gzip.open(gz_path, "rt", encoding="utf-8") as fh:
        for line in fh:
            if not line.strip():
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            word = (obj.get("word") or "").strip()
            if not is_clean_headword(word):
                continue
            glosses = extract_glosses(obj)
            if not glosses:
                continue
            key = norm_key(word, lang)
            if not key:
                continue

            lemma_id += 1
            pos = (obj.get("pos") or "").strip().lower()
            ipa = extract_ipa(obj)
            stressed = extract_ru_stressed(obj, word) if lang == "ru" else ""
            ru_tr = extract_ru_translations(obj) if lang == "en" else None

            lemma_rows.append(
                (
                    lemma_id,
                    lang,
                    key,
                    stressed,
                    ipa,
                    pos,
                    json.dumps(glosses, ensure_ascii=False),
                    ru_tr,
                )
            )
            total_lemmas += 1

            for f in obj.get("forms") or []:
                raw_form = (f.get("form") or "").strip()
                if not raw_form or len(raw_form) > 56 or len(raw_form.split()) > 2:
                    continue
                tags = [str(t) for t in (f.get("tags") or [])]
                if not tags or any(t in SKIP_FORM_TAGS for t in tags):
                    continue
                if any(ord(c) < 0x0400 for c in raw_form) and lang == "ru":
                    # Skip transliterations stored in forms for Russian
                    if not any(0x0400 <= ord(c) <= 0x04FF for c in raw_form):
                        continue
                fkey = norm_key(raw_form, lang)
                if not fkey or fkey == key:
                    continue
                pair = (fkey, lemma_id)
                if pair in seen_forms:
                    continue
                seen_forms.add(pair)
                gram = ", ".join(tags[:3])
                form_rows.append((fkey, lang, lemma_id, gram))
                total_forms += 1

            if len(lemma_rows) >= 25_000:
                cur.executemany(
                    "INSERT INTO lemmas (id, lang, word, stressed, ipa, pos, glosses_json, ru_tr) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    lemma_rows,
                )
                cur.executemany(
                    "INSERT OR IGNORE INTO forms (form, lang, lemma_id, gram) VALUES (?, ?, ?, ?)",
                    form_rows,
                )
                lemma_rows.clear()
                form_rows.clear()
                seen_forms.clear()

    if lemma_rows:
        cur.executemany(
            "INSERT INTO lemmas (id, lang, word, stressed, ipa, pos, glosses_json, ru_tr) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            lemma_rows,
        )
    if form_rows:
        cur.executemany(
            "INSERT OR IGNORE INTO forms (form, lang, lemma_id, gram) VALUES (?, ?, ?, ?)",
            form_rows,
        )
    conn.commit()
    return total_lemmas, total_forms


def main() -> int:
    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    ru_gz = BUILD_DIR / "kaikki-ru.jsonl.gz"
    en_gz = BUILD_DIR / "kaikki-en.jsonl.gz"

    download_if_missing(RU_URL, ru_gz)
    download_if_missing(EN_URL, en_gz)

    if OUT_DB.exists():
        OUT_DB.unlink()

    conn = sqlite3.connect(OUT_DB)
    conn.execute("PRAGMA journal_mode = OFF;")
    conn.execute("PRAGMA synchronous = OFF;")
    conn.execute("PRAGMA temp_store = MEMORY;")
    conn.execute("PRAGMA cache_size = -131072;")
    conn.executescript(
        """
        CREATE TABLE lemmas (
            id INTEGER PRIMARY KEY,
            lang TEXT NOT NULL,
            word TEXT NOT NULL,
            stressed TEXT NOT NULL DEFAULT '',
            ipa TEXT NOT NULL DEFAULT '',
            pos TEXT NOT NULL DEFAULT '',
            glosses_json TEXT NOT NULL,
            ru_tr TEXT
        );
        CREATE TABLE forms (
            form TEXT NOT NULL,
            lang TEXT NOT NULL,
            lemma_id INTEGER NOT NULL,
            gram TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (lang, form, lemma_id)
        ) WITHOUT ROWID;
        """
    )

    ru_l, ru_f = ingest_dump(conn, ru_gz, "ru")
    print(f"Russian ingested: {ru_l:,} lemmas, {ru_f:,} forms")
    en_l, en_f = ingest_dump(conn, en_gz, "en")
    print(f"English ingested: {en_l:,} lemmas, {en_f:,} forms")

    print("Creating index and vacuuming...")
    conn.execute("CREATE INDEX idx_lemmas_lang_word ON lemmas(lang, word);")
    conn.execute("ANALYZE;")
    conn.commit()
    conn.close()

    print(f"Compressing {OUT_DB} ({OUT_DB.stat().st_size / 1e6:.1f} MB) -> {OUT_GZ} ...")
    with open(OUT_DB, "rb") as src, gzip.open(OUT_GZ, "wb", compresslevel=6) as dst:
        shutil.copyfileobj(src, dst, length=1024 * 1024)
    print(f"Done: db={OUT_DB.stat().st_size / 1e6:.1f} MB, gz={OUT_GZ.stat().st_size / 1e6:.1f} MB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
