#!/usr/bin/env python3
"""Build Bookworm's compressed offline EN / RU / ZH-Hans SQLite dictionary packs from Kaikki dumps."""

import argparse
import gzip
import json
import shutil
import sqlite3
import sys
import urllib.request
from pathlib import Path

try:
    from opencc import OpenCC  # type: ignore

    T2S = OpenCC("t2s")
except Exception:
    T2S = None

BUILD_DIR = Path("/tmp/bookworm-dict-build")

EN_PACK_SOURCES = [
    ("ru", "https://kaikki.org/dictionary/Russian/kaikki.org-dictionary-Russian.jsonl.gz", "kaikki-ru.jsonl.gz"),
    ("en", "https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl.gz", "kaikki-en.jsonl.gz"),
    ("zh", "https://kaikki.org/dictionary/Chinese/kaikki.org-dictionary-Chinese.jsonl.gz", "kaikki-zh.jsonl.gz"),
]
RU_WIKT_URL = "https://kaikki.org/ruwiktionary/raw-wiktextract-data.jsonl.gz"
ZH_WIKT_URL = "https://kaikki.org/zhwiktionary/raw-wiktextract-data.jsonl.gz"

SKIP_FORM_TAGS = {
    "table-tags",
    "inflection-template",
    "canonical",
    "romanization",
    "class",
    "entry",
    "archaic",
}

TARGET_LANGS = {"en", "ru", "zh"}


def to_simplified(text: str) -> str:
    if not text or T2S is None:
        return text
    return T2S.convert(text)


def is_cjk_char(ch: str) -> bool:
    o = ord(ch)
    return (0x3400 <= o <= 0x4DBF) or (0x4E00 <= o <= 0x9FFF) or (0xF900 <= o <= 0xFAFF)


def norm_key(word: str, lang: str) -> str:
    w = word.strip().replace("\u0301", "")
    if lang == "zh":
        return "".join(ch for ch in w if not ch.isspace())
    w = w.lower()
    if lang == "ru":
        w = w.replace("ё", "е")
    return w


def download_if_missing(url: str, dest: Path) -> None:
    if dest.exists() and dest.stat().st_size > 1_000_000:
        print(f"Using cached {dest} ({dest.stat().st_size / 1e6:.1f} MB)")
        return
    tmp = dest.with_suffix(".tmp")
    print(f"Downloading {url} -> {dest} ...")
    req = urllib.request.Request(url, headers={"User-Agent": "Bookworm-Dict-Builder/2.0"})
    with urllib.request.urlopen(req, timeout=300) as resp, open(tmp, "wb") as f:
        shutil.copyfileobj(resp, f, length=1024 * 1024)
    tmp.replace(dest)
    print(f"Downloaded {dest.stat().st_size / 1e6:.1f} MB")


def extract_ipa(obj: dict, lang: str) -> str:
    if lang == "zh":
        for snd in obj.get("sounds") or []:
            tags = set(snd.get("tags") or [])
            zp = snd.get("zh_pron")
            if isinstance(zp, str) and zp.strip() and ("Pinyin" in tags or "Standard-Chinese" in tags):
                return zp.strip()
    for snd in obj.get("sounds") or []:
        ipa = snd.get("ipa")
        if isinstance(ipa, str) and ipa.strip():
            return ipa.strip().strip("/")
    return ""


def extract_ru_stressed(obj: dict) -> str:
    for f in obj.get("forms") or []:
        tags = set(f.get("tags") or [])
        if "canonical" in tags:
            val = (f.get("form") or "").strip()
            if "\u0301" in val:
                return val
    return ""


def is_boilerplate_gloss(g: str) -> bool:
    t = g.strip()
    return (
        not t
        or "__NOTITLECONVERT__" in t
        or "__NOEDITSECTION__" in t
        or t.startswith("关于「")
        or "请见「" in t
        or "For pronunciation and definitions" in t
        or "пока не переведено" in t
        or "Это слово или выражение" in t
    )


def extract_glosses(obj: dict, convert_simplified: bool = False) -> list[str]:
    real_out: list[str] = []
    seen: set[str] = set()
    for sense in obj.get("senses") or []:
        if len(real_out) >= 4:
            break
        tags = set(sense.get("tags") or [])
        if "no-gloss" in tags or "form-of" in tags or sense.get("form_of"):
            continue
        raw_glosses = sense.get("glosses") or []
        if not raw_glosses:
            continue
        g = " — ".join(str(x).strip() for x in raw_glosses if str(x).strip())
        if is_boilerplate_gloss(g):
            continue
        if convert_simplified:
            g = to_simplified(g)
        if len(g) > 180:
            g = g[:177].rstrip() + "..."
        low = g.lower()
        if low not in seen:
            seen.add(low)
            real_out.append(g)
    return real_out


def extract_form_of_targets(obj: dict, lang: str, self_key: str) -> list[tuple[str, str]]:
    """Extracts `(target_lemma_key, gram)` links for pure form-of entries."""
    out: list[tuple[str, str]] = []
    seen_targets: set[str] = set()
    for sense in obj.get("senses") or []:
        form_of = sense.get("form_of") or []
        if not form_of:
            continue
        raw_tags = [str(t).strip() for t in (sense.get("tags") or []) if str(t).strip() not in ("", "form-of", "no-gloss")]
        gram = ", ".join(raw_tags[:3])
        for fo in form_of:
            tw_raw = str((fo or {}).get("word") or "").strip()
            if not tw_raw:
                continue
            tw = norm_key(tw_raw, lang)
            if lang == "zh":
                tw = norm_key(to_simplified(tw), "zh")
            if not tw or tw == self_key or tw in seen_targets:
                continue
            seen_targets.add(tw)
            out.append((tw, gram))
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


def is_clean_headword(word: str, lang: str) -> bool:
    if not word:
        return False
    if lang == "zh":
        compact = "".join(ch for ch in word if not ch.isspace())
        return 1 <= len(compact) <= 8 and any(is_cjk_char(ch) for ch in compact)
    if len(word) > 48 or len(word.split()) > 2:
        return False
    return any(ch.isalpha() for ch in word)


def init_db(db_path: Path) -> sqlite3.Connection:
    if db_path.exists():
        db_path.unlink()
    conn = sqlite3.connect(db_path)
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
        CREATE TEMP TABLE form_links (
            form TEXT NOT NULL,
            lang TEXT NOT NULL,
            target_word TEXT NOT NULL,
            gram TEXT NOT NULL DEFAULT ''
        );
        """
    )
    return conn


def flush_batches(cur: sqlite3.Cursor, lemma_rows: list, form_rows: list, form_link_rows: list) -> None:
    if lemma_rows:
        cur.executemany(
            "INSERT INTO lemmas (id, lang, word, stressed, ipa, pos, glosses_json, ru_tr) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            lemma_rows,
        )
        lemma_rows.clear()
    if form_rows:
        cur.executemany(
            "INSERT OR IGNORE INTO forms (form, lang, lemma_id, gram) VALUES (?, ?, ?, ?)",
            form_rows,
        )
        form_rows.clear()
    if form_link_rows:
        cur.executemany(
            "INSERT INTO form_links (form, lang, target_word, gram) VALUES (?, ?, ?, ?)",
            form_link_rows,
        )
        form_link_rows.clear()


def ingest_dump(
    conn: sqlite3.Connection,
    gz_path: Path,
    fixed_lang: str | None,
    convert_zh_glosses: bool = False,
) -> tuple[int, int]:
    cur = conn.cursor()
    lemma_rows = []
    form_rows = []
    form_link_rows = []
    next_id_row = cur.execute("SELECT COALESCE(MAX(id), 0) FROM lemmas").fetchone()
    lemma_id = int(next_id_row[0])

    total_lemmas = 0
    total_forms = 0
    seen_forms: set[tuple[str, str, int]] = set()

    with gzip.open(gz_path, "rt", encoding="utf-8") as fh:
        for line in fh:
            if not line.strip():
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue

            lang = fixed_lang or (obj.get("lang_code") or "").strip().lower()
            if lang not in TARGET_LANGS:
                continue

            word = (obj.get("word") or "").strip()
            if not is_clean_headword(word, lang):
                continue

            orig_key = norm_key(word, lang)
            if not orig_key:
                continue
            key = norm_key(to_simplified(orig_key), "zh") if lang == "zh" else orig_key
            if not key:
                continue

            glosses = extract_glosses(obj, convert_simplified=convert_zh_glosses)
            if not glosses:
                for target_word, gram in extract_form_of_targets(obj, lang, key):
                    form_link_rows.append((key, lang, target_word, gram))
                    if lang == "zh" and orig_key != key:
                        form_link_rows.append((orig_key, lang, target_word, gram))
                if len(form_link_rows) >= 50_000:
                    flush_batches(cur, lemma_rows, form_rows, form_link_rows)
                continue

            lemma_id += 1
            pos = (obj.get("pos") or "").strip().lower()
            ipa = extract_ipa(obj, lang)
            stressed = extract_ru_stressed(obj) if lang == "ru" else ""
            ru_tr = extract_ru_translations(obj) if (lang == "en" and fixed_lang == "en") else None

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

            # Map Traditional / alternate Chinese script form -> Simplified lemma
            if lang == "zh" and orig_key != key:
                pair = (orig_key, "zh", lemma_id)
                seen_forms.add(pair)
                form_rows.append((orig_key, "zh", lemma_id, "Traditional"))
                total_forms += 1

            for f in obj.get("forms") or []:
                raw_form = (f.get("form") or "").strip()
                if not raw_form or len(raw_form) > 56 or len(raw_form.split()) > 2:
                    continue
                tags = [str(t) for t in (f.get("tags") or [])]
                if not tags or any(t in SKIP_FORM_TAGS for t in tags):
                    continue
                if lang == "ru" and any(ord(c) < 0x0400 for c in raw_form):
                    if not any(0x0400 <= ord(c) <= 0x04FF for c in raw_form):
                        continue
                if lang == "zh" and not any(is_cjk_char(c) for c in raw_form):
                    continue
                fkey = norm_key(raw_form, lang)
                if not fkey or fkey == key:
                    continue
                pair = (fkey, lang, lemma_id)
                if pair in seen_forms:
                    continue
                seen_forms.add(pair)
                gram = ", ".join(tags[:3])
                form_rows.append((fkey, lang, lemma_id, gram))
                total_forms += 1

            if len(lemma_rows) >= 25_000:
                flush_batches(cur, lemma_rows, form_rows, form_link_rows)
                seen_forms.clear()

    flush_batches(cur, lemma_rows, form_rows, form_link_rows)
    conn.commit()
    return total_lemmas, total_forms


def finalize_and_compress(conn: sqlite3.Connection, db_path: Path, gz_path: Path) -> None:
    print(f"Creating index for {db_path.name}...")
    conn.execute("CREATE INDEX idx_lemmas_lang_word ON lemmas(lang, word);")
    print(f"Resolving deferred form-of links for {db_path.name}...")
    conn.execute(
        """
        INSERT OR IGNORE INTO forms (form, lang, lemma_id, gram)
        SELECT fl.form, fl.lang, l.id, fl.gram
        FROM form_links fl
        JOIN lemmas l ON l.lang = fl.lang AND l.word = fl.target_word;
        """
    )
    conn.execute("DROP TABLE IF EXISTS form_links;")
    conn.execute("ANALYZE;")
    conn.commit()
    conn.close()

    print(f"Compressing {db_path} ({db_path.stat().st_size / 1e6:.1f} MB) -> {gz_path} ...")
    with open(db_path, "rb") as src, gzip.open(gz_path, "wb", compresslevel=6) as dst:
        shutil.copyfileobj(src, dst, length=1024 * 1024)
    print(f"Done: db={db_path.stat().st_size / 1e6:.1f} MB, gz={gz_path.stat().st_size / 1e6:.1f} MB")


def build_pack_en() -> None:
    out_db = BUILD_DIR / "bookworm-dict-def-en.db"
    out_gz = BUILD_DIR / "bookworm-dict-def-en.db.gz"
    conn = init_db(out_db)
    for lang, url, fname in EN_PACK_SOURCES:
        src = BUILD_DIR / fname
        download_if_missing(url, src)
        l, f = ingest_dump(conn, src, fixed_lang=lang, convert_zh_glosses=False)
        print(f"[def-en] {lang}: {l:,} lemmas, {f:,} forms")
    finalize_and_compress(conn, out_db, out_gz)


def build_pack_ru() -> None:
    out_db = BUILD_DIR / "bookworm-dict-def-ru.db"
    out_gz = BUILD_DIR / "bookworm-dict-def-ru.db.gz"
    src = BUILD_DIR / "kaikki-ruwikt-raw.jsonl.gz"
    download_if_missing(RU_WIKT_URL, src)
    conn = init_db(out_db)
    l, f = ingest_dump(conn, src, fixed_lang=None, convert_zh_glosses=False)
    print(f"[def-ru] total (en/ru/zh): {l:,} lemmas, {f:,} forms")
    finalize_and_compress(conn, out_db, out_gz)


def build_pack_zh() -> None:
    out_db = BUILD_DIR / "bookworm-dict-def-zh.db"
    out_gz = BUILD_DIR / "bookworm-dict-def-zh.db.gz"
    src = BUILD_DIR / "kaikki-zhwikt-raw.jsonl.gz"
    download_if_missing(ZH_WIKT_URL, src)
    conn = init_db(out_db)
    l, f = ingest_dump(conn, src, fixed_lang=None, convert_zh_glosses=True)
    print(f"[def-zh] total (en/ru/zh Simplified): {l:,} lemmas, {f:,} forms")
    finalize_and_compress(conn, out_db, out_gz)


def main() -> int:
    parser = argparse.ArgumentParser(description="Build Bookworm offline dictionary packs.")
    parser.add_argument(
        "--pack",
        choices=["en", "ru", "zh", "all"],
        default="all",
        help="Which definition-language pack to build",
    )
    args = parser.parse_args()
    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    if args.pack in ("en", "all"):
        build_pack_en()
    if args.pack in ("ru", "all"):
        build_pack_ru()
    if args.pack in ("zh", "all"):
        build_pack_zh()
    return 0


if __name__ == "__main__":
    sys.exit(main())
