use std::collections::BTreeSet;

use crate::bounded_text;

#[cfg(any(target_os = "windows", test))]
fn windows_family_name(registry_name: &str) -> Option<String> {
    let name = registry_name.trim().trim_start_matches('@').trim();
    if name.is_empty() {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    let suffixes = [" (truetype)", " (opentype)", " (all res)"];
    let family = suffixes
        .iter()
        .find(|suffix| lower.ends_with(**suffix))
        .map(|suffix| &name[..name.len() - suffix.len()])
        .unwrap_or(name)
        .trim();
    (!family.is_empty()).then(|| bounded_text(family, 256))
}

fn list_blocking() -> Result<Vec<String>, String> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let program = if std::path::Path::new("/usr/bin/fc-list").is_file() {
            "/usr/bin/fc-list"
        } else {
            "fc-list"
        };
        let output = Command::new(program)
            .arg("--format=%{family}\n")
            .output()
            .map_err(|error| format!("failed to execute fc-list: {error}"))?;
        if !output.status.success() {
            return Err("fc-list returned a non-zero status".to_string());
        }

        let mut families = BTreeSet::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            for family in line.split(',') {
                let family = family.trim();
                if !family.is_empty() {
                    families.insert(bounded_text(family, 256));
                }
            }
        }
        Ok(families.into_iter().collect())
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::{
            enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ},
            RegKey,
        };

        const FONT_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";
        let mut families = BTreeSet::new();
        for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            let Ok(key) = RegKey::predef(root).open_subkey_with_flags(FONT_KEY, KEY_READ) else {
                continue;
            };
            for value in key.enum_values().flatten() {
                if let Some(family) = windows_family_name(&value.0) {
                    families.insert(family);
                }
            }
        }
        Ok(families.into_iter().collect())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub(crate) async fn list_system_fonts() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(list_blocking)
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::windows_family_name;

    #[test]
    fn windows_registry_font_names_keep_the_css_family_and_drop_type_suffixes() {
        assert_eq!(
            windows_family_name("Noto Sans CJK SC Bold (TrueType)").as_deref(),
            Some("Noto Sans CJK SC Bold")
        );
        assert_eq!(
            windows_family_name("@Vertical Font").as_deref(),
            Some("Vertical Font")
        );
        assert_eq!(windows_family_name("  "), None);
    }
}
