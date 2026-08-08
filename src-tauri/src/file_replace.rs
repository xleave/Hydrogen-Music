use std::{
    fs,
    path::{Path, PathBuf},
};

fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = path.file_name().unwrap_or_default().to_os_string();
    file_name.push(suffix);
    path.with_file_name(file_name)
}

fn backup_path(destination: &Path) -> PathBuf {
    sibling_with_suffix(destination, ".bak")
}

pub fn recover_backup(destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Ok(());
    }
    let backup = backup_path(destination);
    if backup.is_file() {
        fs::rename(backup, destination).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(any(windows, test))]
fn replace_with_backup(temporary: &Path, destination: &Path) -> Result<(), String> {
    let backup = backup_path(destination);
    if backup.exists() {
        fs::remove_file(&backup).map_err(|error| error.to_string())?;
    }
    if destination.exists() {
        fs::rename(destination, &backup).map_err(|error| error.to_string())?;
    }
    if let Err(error) = fs::rename(temporary, destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(error.to_string());
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

pub fn replace(temporary: &Path, destination: &Path) -> Result<(), String> {
    recover_backup(destination)?;

    #[cfg(windows)]
    return replace_with_backup(temporary, destination);

    #[cfg(not(windows))]
    fs::rename(temporary, destination).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "hydrogen-music-file-replace-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn backup_replacement_keeps_the_new_file() {
        let directory = TestDirectory::new();
        let destination = directory.0.join("settings.json");
        let temporary = directory.0.join("settings.tmp");
        fs::write(&destination, b"old").unwrap();
        fs::write(&temporary, b"new").unwrap();

        replace_with_backup(&temporary, &destination).unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"new");
        assert!(!backup_path(&destination).exists());
    }

    #[test]
    fn missing_destination_is_restored_from_backup() {
        let directory = TestDirectory::new();
        let destination = directory.0.join("last-playlist.json");
        let backup = backup_path(&destination);
        fs::write(&backup, b"saved state").unwrap();

        recover_backup(&destination).unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"saved state");
        assert!(!backup.exists());
    }
}
