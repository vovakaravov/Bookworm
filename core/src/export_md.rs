use crate::atomic_write_string;
use crate::library::{BookRecord, sanitize_filename_segment};
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

fn yaml_escape(raw: &str) -> String {
    let clean = raw.replace('\r', " ").replace('\n', " ");
    format!("\"{}\"", clean.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Renders a single book's highlights, margin notes, and bookmarks as clean Markdown
/// with YAML frontmatter suitable for Obsidian / VKarma note vaults.
pub fn render_book_notes_markdown(rec: &BookRecord) -> String {
    let mut out = String::new();
    let title = rec.display_title();
    let authors = rec.authors_display();
    let tags = rec.all_tags();
    let pct = rec.progress_pct();

    out.push_str("---\n");
    out.push_str(&format!("title: {}\n", yaml_escape(title)));
    out.push_str(&format!("authors: {}\n", yaml_escape(&authors)));
    if let Some(series) = rec.series_badge() {
        out.push_str(&format!("series: {}\n", yaml_escape(&series)));
    }
    out.push_str(&format!("status: {}\n", rec.user.status.label()));
    if rec.user.rating > 0 {
        out.push_str(&format!("rating: {}\n", rec.user.rating));
    }
    out.push_str(&format!("progress: {}%\n", pct));
    if !tags.is_empty() {
        out.push_str("tags:\n");
        for tag in &tags {
            out.push_str(&format!("  - {}\n", yaml_escape(tag)));
        }
    }
    out.push_str("source: bookworm\n");
    out.push_str("---\n\n");

    out.push_str(&format!("# {title}\n\n"));
    if rec.series_badge().is_some() {
        out.push_str(&format!(
            "*{authors}* · {} · Progress: {pct}%\n\n",
            rec.series_badge().unwrap_or_default()
        ));
    } else {
        out.push_str(&format!("*{authors}* · Progress: {pct}%\n\n"));
    }

    if !rec.user.highlights.is_empty() {
        let mut by_chapter: BTreeMap<usize, Vec<&crate::library::BookHighlight>> = BTreeMap::new();
        for hl in &rec.user.highlights {
            by_chapter.entry(hl.chapter_idx).or_default().push(hl);
        }
        for (ch_idx, mut hls) in by_chapter {
            hls.sort_by_key(|h| (h.start_block, h.start_offset, h.created_at.as_str()));
            let ch_title = hls
                .iter()
                .find_map(|h| {
                    let t = h.chapter_title.trim();
                    (!t.is_empty()).then_some(t)
                })
                .unwrap_or("Chapter");
            if ch_title.eq_ignore_ascii_case("Chapter") {
                out.push_str(&format!("## Chapter {}\n\n", ch_idx + 1));
            } else {
                out.push_str(&format!("## Ch {}: {}\n\n", ch_idx + 1, ch_title));
            }
            for hl in hls {
                let quote = hl.quote.trim();
                if quote.is_empty() {
                    continue;
                }
                for line in quote.lines() {
                    out.push_str("> ");
                    out.push_str(line.trim());
                    out.push('\n');
                }
                let style_meta = format!(
                    "> — *{} · {}*",
                    hl.color.label(),
                    hl.style.label()
                );
                out.push_str(&style_meta);
                out.push_str("\n\n");
                let note = hl.note.trim();
                if !note.is_empty() {
                    out.push_str("**Note:** ");
                    out.push_str(note);
                    out.push_str("\n\n");
                }
            }
        }
    }

    if !rec.user.bookmarks.is_empty() {
        let mut bms: Vec<&crate::library::BookBookmark> = rec.user.bookmarks.iter().collect();
        bms.sort_by_key(|b| (b.chapter_idx, b.scroll_pct.to_bits()));
        out.push_str("## Bookmarks\n\n");
        for bm in bms {
            let b_pct = (bm.scroll_pct.clamp(0.0, 1.0) * 100.0).round() as u8;
            let ch_name = if bm.chapter_title.trim().is_empty() {
                format!("Chapter {}", bm.chapter_idx + 1)
            } else {
                format!("Ch {} · {}", bm.chapter_idx + 1, bm.chapter_title.trim())
            };
            if bm.note.trim().is_empty() {
                out.push_str(&format!("- **{ch_name}** ({b_pct}%)\n"));
            } else {
                out.push_str(&format!("- **{ch_name}** ({b_pct}%): {}\n", bm.note.trim()));
            }
        }
        out.push('\n');
    }

    let trimmed = out.trim_end();
    format!("{trimmed}\n")
}

/// Exports Markdown notes for all books in `books` that have highlights or bookmarks into `out_dir`.
/// Returns the number of Markdown files written.
pub fn export_library_notes_to_dir(books: &[BookRecord], out_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(out_dir)?;
    let mut written = 0usize;
    let mut used_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for rec in books {
        if rec.user.highlights.is_empty() && rec.user.bookmarks.is_empty() {
            continue;
        }
        let mut stem = sanitize_filename_segment(rec.display_title());
        if stem.is_empty() {
            stem = "Book Notes".to_string();
        }
        let mut filename = format!("{stem}.md");
        let mut suffix = 2usize;
        while !used_names.insert(filename.to_ascii_lowercase()) {
            filename = format!("{stem} ({suffix}).md");
            suffix += 1;
        }
        let md = render_book_notes_markdown(rec);
        atomic_write_string(&out_dir.join(filename), &md)?;
        written += 1;
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epub::EpubMetadata;
    use crate::library::{
        BookBookmark, BookFormat, BookHighlight, BookUserState, HighlightColor,
        HighlightDecoration, ReadingStatus,
    };
    use std::path::PathBuf;

    fn sample_record(id_suffix: &str, with_notes: bool) -> BookRecord {
        let mut user = BookUserState {
            status: ReadingStatus::Reading,
            rating: 5,
            overall_pct: 0.42,
            ..Default::default()
        };
        if with_notes {
            user.highlights.push(BookHighlight {
                id: format!("hl-{id_suffix}"),
                chapter_idx: 2,
                chapter_title: "Part Three".into(),
                start_block: 4,
                start_offset: 0,
                end_block: 4,
                end_offset: 20,
                quote: "Plain files outlive platforms.".into(),
                color: HighlightColor::Purple,
                style: HighlightDecoration::Squiggly,
                note: "Keep this principle central.".into(),
                created_at: "2026-11-25T00:00:00Z".into(),
                updated_at: "2026-11-25T00:00:00Z".into(),
            });
            user.bookmarks.push(BookBookmark {
                id: format!("bm-{id_suffix}"),
                chapter_idx: 2,
                chapter_title: "Part Three".into(),
                scroll_pct: 0.5,
                note: "Key thesis".into(),
                created_at: "2026-11-25T00:00:00Z".into(),
            });
        }
        BookRecord {
            path: PathBuf::from(format!("/lib/Book{id_suffix}.epub")),
            rel_path: format!("Book{id_suffix}.epub"),
            format: BookFormat::Epub,
            file_size_bytes: 1024,
            chapter_count: 10,
            metadata: EpubMetadata {
                title: format!("Zen Mind {id_suffix}"),
                authors: vec!["Shunryu Suzuki".into()],
                subjects: vec!["Zen".into(), "Philosophy".into()],
                ..Default::default()
            },
            cover: None,
            cover_cache_path: None,
            user,
        }
    }

    #[test]
    fn markdown_export_renders_notes_and_skips_unannotated_books() {
        let tmp = tempfile::tempdir().unwrap();
        let annotated = sample_record("One", true);
        let empty = sample_record("Empty", false);

        let count = export_library_notes_to_dir(&[annotated, empty], tmp.path()).unwrap();
        assert_eq!(count, 1);

        let exported_path = tmp.path().join("Zen Mind One.md");
        assert!(exported_path.is_file());
        assert!(!tmp.path().join("Zen Mind Empty.md").exists());

        let md = std::fs::read_to_string(&exported_path).unwrap();
        assert!(md.contains("title: \"Zen Mind One\""));
        assert!(md.contains("rating: 5"));
        assert!(md.contains("source: bookworm"));
        assert!(md.contains("## Ch 3: Part Three"));
        assert!(md.contains("> Plain files outlive platforms."));
        assert!(md.contains("*Purple · Squiggly*"));
        assert!(md.contains("**Note:** Keep this principle central."));
        assert!(md.contains("- **Ch 3 · Part Three** (50%): Key thesis"));
    }
}
