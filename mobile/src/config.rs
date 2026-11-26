use anyhow::Result;
use bookworm_core::atomic_write_string;
use bookworm_core::library::{detect_default_books_root, discover_books_root_candidates, library_state_root_for};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn app_storage_dir(subdir: &str) -> PathBuf {
    #[cfg(target_os = "android")]
    let base = gpui_mobile::packages::path_provider::documents_directory()
        .unwrap_or_else(|_| PathBuf::from("/data/local/tmp/bookworm-mobile"));

    #[cfg(not(target_os = "android"))]
    let base = std::env::temp_dir().join("bookworm-mobile");

    let target = if subdir.trim().is_empty() {
        base
    } else {
        base.join(subdir)
    };
    let _ = fs::create_dir_all(&target);
    target
}

pub fn app_cache_dir(subdir: &str) -> PathBuf {
    #[cfg(target_os = "android")]
    let base = gpui_mobile::packages::path_provider::cache_directory()
        .unwrap_or_else(|_| app_storage_dir("cache"));

    #[cfg(not(target_os = "android"))]
    let base = std::env::temp_dir().join("bookworm-mobile-cache");

    let target = if subdir.trim().is_empty() {
        base
    } else {
        base.join(subdir)
    };
    let _ = fs::create_dir_all(&target);
    target
}

pub fn has_external_storage_access() -> bool {
    #[cfg(target_os = "android")]
    {
        crate::has_all_files_access()
    }
    #[cfg(not(target_os = "android"))]
    {
        true
    }
}

pub fn default_books_root() -> PathBuf {
    let fallback = app_storage_dir("books");
    detect_default_books_root(&fallback)
}

pub fn state_root_for(books_root: &Path) -> PathBuf {
    library_state_root_for(books_root, &app_storage_dir("books"))
}

pub fn books_root_candidates(current: &Path) -> Vec<(String, PathBuf)> {
    discover_books_root_candidates(current, &app_storage_dir("books"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LibraryGridSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl LibraryGridSize {
    pub fn all() -> &'static [LibraryGridSize] {
        &[Self::Small, Self::Medium, Self::Large]
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Small => "S",
            Self::Medium => "M",
            Self::Large => "L",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Small => "sm",
            Self::Medium => "md",
            Self::Large => "lg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LibrarySort {
    #[default]
    Recent,
    Title,
    Author,
    Progress,
}

impl LibrarySort {
    pub fn all() -> &'static [LibrarySort] {
        &[Self::Recent, Self::Title, Self::Author, Self::Progress]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recently Read",
            Self::Title => "Title",
            Self::Author => "Author",
            Self::Progress => "Progress",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Recent => "recent",
            Self::Title => "title",
            Self::Author => "author",
            Self::Progress => "progress",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BookwormMobileConfig {
    pub books_root: Option<PathBuf>,
    pub two_column_spread: bool,
    pub library_grid_size: LibraryGridSize,
    pub library_sort: LibrarySort,
}

impl Default for BookwormMobileConfig {
    fn default() -> Self {
        Self {
            books_root: None,
            two_column_spread: true,
            library_grid_size: LibraryGridSize::Medium,
            library_sort: LibrarySort::Recent,
        }
    }
}

impl BookwormMobileConfig {
    pub fn state_path() -> PathBuf {
        app_storage_dir("").join("bookworm-state.json")
    }

    pub fn load() -> Self {
        let path = Self::state_path();
        if let Ok(raw) = fs::read_to_string(&path)
            && let Ok(cfg) = serde_json::from_str::<Self>(&raw)
        {
            return cfg;
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        atomic_write_string(&Self::state_path(), &json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_config_round_trips_json() {
        let cfg = BookwormMobileConfig {
            books_root: Some(PathBuf::from("/sdcard/Documents/Library")),
            two_column_spread: false,
            library_grid_size: LibraryGridSize::Large,
            library_sort: LibrarySort::Title,
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: BookwormMobileConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.books_root, cfg.books_root);
        assert!(!back.two_column_spread);
        assert_eq!(back.library_grid_size, LibraryGridSize::Large);
        assert_eq!(back.library_sort, LibrarySort::Title);
    }
}
