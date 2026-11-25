use anyhow::Result;
use bookworm_core::atomic_write_string;
use bookworm_core::library::detect_default_books_root;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BookwormConfig {
    pub library_root: Option<PathBuf>,
    pub export_notes_dir: Option<PathBuf>,
    pub outline_open: bool,
}

impl Default for BookwormConfig {
    fn default() -> Self {
        Self {
            library_root: None,
            export_notes_dir: None,
            outline_open: true,
        }
    }
}

impl BookwormConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("bookworm")
            .join("config.json")
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        if let Ok(raw) = fs::read_to_string(&path)
            && let Ok(cfg) = serde_json::from_str::<Self>(&raw)
        {
            return cfg;
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        atomic_write_string(&path, &json)?;
        Ok(())
    }

    pub fn fallback_books_dir() -> PathBuf {
        dirs::document_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
    }

    pub fn effective_library_root(&self) -> PathBuf {
        if let Some(configured) = &self.library_root
            && configured.is_dir()
        {
            return configured.clone();
        }
        detect_default_books_root(&Self::fallback_books_dir())
    }

    pub fn effective_export_dir(&self) -> PathBuf {
        if let Some(configured) = &self.export_notes_dir {
            return configured.clone();
        }
        default_export_dir(&self.effective_library_root())
    }
}

pub fn default_export_dir(library_root: &Path) -> PathBuf {
    library_root.join("Bookworm Notes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_json() {
        let cfg = BookwormConfig {
            library_root: Some(PathBuf::from("/tmp/books")),
            export_notes_dir: Some(PathBuf::from("/tmp/notes")),
            outline_open: false,
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let parsed: BookwormConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.library_root, cfg.library_root);
        assert_eq!(parsed.export_notes_dir, cfg.export_notes_dir);
        assert!(!parsed.outline_open);
    }
}
