//! Basic workspace file operations.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Validate a single file or folder name.
pub fn valid_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

/// Create an empty file inside `parent`.
pub fn create_file(parent: &Path, name: &str) -> io::Result<PathBuf> {
    if !valid_name(name) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"));
    }
    let path = parent.join(name.trim());
    fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
    Ok(path)
}

/// Create a folder inside `parent`.
pub fn create_folder(parent: &Path, name: &str) -> io::Result<PathBuf> {
    if !valid_name(name) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid folder name"));
    }
    let path = parent.join(name.trim());
    fs::create_dir(&path)?;
    Ok(path)
}

/// Rename an existing file or folder, returning the new path.
pub fn rename(path: &Path, new_name: &str) -> io::Result<PathBuf> {
    if !valid_name(new_name) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid name"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path has no parent"))?;
    let target = parent.join(new_name.trim());
    if target != path && target.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the name is already in use",
        ));
    }
    fs::rename(path, &target)?;
    Ok(target)
}

/// Delete a file, or a folder and its contents.
pub fn delete(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Reveal a path in the platform file manager.
pub fn reveal(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let target = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        Command::new("xdg-open").arg(target).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg("-R").arg(path).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_names() {
        assert!(valid_name("notes.md"));
        assert!(!valid_name(""));
        assert!(!valid_name(".."));
        assert!(!valid_name("a/b"));
        assert!(!valid_name("a\\b"));
    }

    #[test]
    fn creates_renames_and_deletes_entries() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = create_file(dir.path(), "draft.md").expect("create file");
        assert!(file.is_file());
        assert!(create_file(dir.path(), "draft.md").is_err());

        let renamed = rename(&file, "final.md").expect("rename");
        assert!(renamed.is_file());
        assert!(!file.exists());

        let folder = create_folder(dir.path(), "notes").expect("create folder");
        fs::write(folder.join("one.md"), "one").expect("write");
        delete(&folder).expect("delete folder");
        assert!(!folder.exists());
    }
}
