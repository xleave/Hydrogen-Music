pub use wayclip_global_hotkey::{hotkey, GlobalHotKeyEvent, HotKeyState};

use std::fmt;
use wayclip_global_hotkey::{hotkey::HotKey, GlobalHotKeyManager as InnerManager};

#[derive(Debug, Clone)]
pub struct Error(String);

impl Error {
    fn unavailable(detail: impl Into<String>) -> Self {
        Self(detail.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<wayclip_global_hotkey::Error> for Error {
    fn from(error: wayclip_global_hotkey::Error) -> Self {
        Self(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// API-compatible manager used by tauri-plugin-global-shortcut.
///
/// Wayclip selects X11 or the XDG GlobalShortcuts portal at runtime. Backend
/// initialization is deliberately non-fatal: a session without a compatible
/// portal must still be able to start Hydrogen Music. Registration then returns
/// the original backend error so the settings UI can report that global
/// shortcuts are unavailable instead of pretending they were enabled.
pub struct GlobalHotKeyManager {
    inner: Option<InnerManager>,
    init_error: Option<String>,
}

impl GlobalHotKeyManager {
    pub fn new() -> Result<Self> {
        if std::env::var_os("GLOBAL_HOTKEY_APP_ID").is_none() {
            std::env::set_var("GLOBAL_HOTKEY_APP_ID", "music.hydrogen.local");
        }
        match InnerManager::new() {
            Ok(inner) => Ok(Self {
                inner: Some(inner),
                init_error: None,
            }),
            Err(error) => {
                let detail = format!("global shortcut backend unavailable: {error}");
                eprintln!("[global shortcut] {detail}");
                Ok(Self {
                    inner: None,
                    init_error: Some(detail),
                })
            }
        }
    }

    fn inner(&self) -> Result<&InnerManager> {
        self.inner.as_ref().ok_or_else(|| {
            Error::unavailable(
                self.init_error
                    .clone()
                    .unwrap_or_else(|| "global shortcut backend unavailable".to_string()),
            )
        })
    }

    pub fn register(&self, hotkey: HotKey) -> Result<()> {
        self.inner()?.register(hotkey).map_err(Into::into)
    }

    pub fn unregister(&self, hotkey: HotKey) -> Result<()> {
        self.inner()?.unregister(hotkey).map_err(Into::into)
    }

    pub fn register_all(&self, hotkeys: &[HotKey]) -> Result<()> {
        self.inner()?.register_all(hotkeys).map_err(Into::into)
    }

    pub fn unregister_all(&self, hotkeys: &[HotKey]) -> Result<()> {
        if hotkeys.is_empty() && self.inner.is_none() {
            return Ok(());
        }
        self.inner()?.unregister_all(hotkeys).map_err(Into::into)
    }
}
