pub use wayclip_global_hotkey::{hotkey, GlobalHotKeyEvent, HotKeyState};

use std::{fmt, sync::Mutex};
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

#[derive(Default)]
struct BackendState {
    inner: Option<InnerManager>,
    init_error: Option<String>,
}

/// API-compatible manager used by tauri-plugin-global-shortcut.
///
/// The important lifecycle rule is that constructing the Tauri plugin must not
/// touch the XDG GlobalShortcuts portal. On some Wayland sessions portal setup
/// can block while the WebView is still starting, which leaves a visible but
/// completely white application window. The actual backend is therefore
/// created only when the renderer asks to register a global shortcut, after the
/// Vue application has already mounted.
pub struct GlobalHotKeyManager {
    state: Mutex<BackendState>,
}

impl GlobalHotKeyManager {
    pub fn new() -> Result<Self> {
        if std::env::var_os("GLOBAL_HOTKEY_APP_ID").is_none() {
            std::env::set_var("GLOBAL_HOTKEY_APP_ID", "music.hydrogen.local");
        }
        Ok(Self {
            state: Mutex::new(BackendState::default()),
        })
    }

    fn with_inner<T>(
        &self,
        operation: impl FnOnce(&InnerManager) -> std::result::Result<T, wayclip_global_hotkey::Error>,
    ) -> Result<T> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| Error::unavailable("global shortcut backend lock was poisoned"))?;

        if state.inner.is_none() {
            match InnerManager::new() {
                Ok(inner) => {
                    state.inner = Some(inner);
                    state.init_error = None;
                }
                Err(error) => {
                    let detail = format!("global shortcut backend unavailable: {error}");
                    state.init_error = Some(detail.clone());
                    eprintln!("[global shortcut] {detail}");
                    return Err(Error::unavailable(detail));
                }
            }
        }

        let inner = state.inner.as_ref().ok_or_else(|| {
            Error::unavailable(
                state
                    .init_error
                    .clone()
                    .unwrap_or_else(|| "global shortcut backend unavailable".to_string()),
            )
        })?;
        operation(inner).map_err(Into::into)
    }

    pub fn register(&self, hotkey: HotKey) -> Result<()> {
        self.with_inner(|inner| inner.register(hotkey))
    }

    pub fn unregister(&self, hotkey: HotKey) -> Result<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| Error::unavailable("global shortcut backend lock was poisoned"))?;
        let Some(inner) = state.inner.as_ref() else {
            return Ok(());
        };
        inner.unregister(hotkey).map_err(Into::into)
    }

    pub fn register_all(&self, hotkeys: &[HotKey]) -> Result<()> {
        if hotkeys.is_empty() {
            return Ok(());
        }
        self.with_inner(|inner| inner.register_all(hotkeys))
    }

    pub fn unregister_all(&self, hotkeys: &[HotKey]) -> Result<()> {
        if hotkeys.is_empty() {
            return Ok(());
        }
        let state = self
            .state
            .lock()
            .map_err(|_| Error::unavailable("global shortcut backend lock was poisoned"))?;
        let Some(inner) = state.inner.as_ref() else {
            return Ok(());
        };
        inner.unregister_all(hotkeys).map_err(Into::into)
    }
}
