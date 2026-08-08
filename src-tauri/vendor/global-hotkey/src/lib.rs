pub use wayclip_global_hotkey::{
    hotkey, Error, GlobalHotKeyEvent, GlobalHotKeyEventReceiver, HotKeyState, Result,
};

use wayclip_global_hotkey::{hotkey::HotKey, GlobalHotKeyManager as InnerManager};

/// API-compatible manager used by tauri-plugin-global-shortcut.
/// Wayclip selects X11 or the XDG GlobalShortcuts portal at runtime.
pub struct GlobalHotKeyManager {
    inner: InnerManager,
}

impl GlobalHotKeyManager {
    pub fn new() -> Result<Self> {
        if std::env::var_os("GLOBAL_HOTKEY_APP_ID").is_none() {
            std::env::set_var("GLOBAL_HOTKEY_APP_ID", "music.hydrogen.local");
        }
        InnerManager::new().map(|inner| Self { inner })
    }

    pub fn register(&self, hotkey: HotKey) -> Result<()> {
        self.inner.register(hotkey)
    }

    pub fn unregister(&self, hotkey: HotKey) -> Result<()> {
        self.inner.unregister(hotkey)
    }

    pub fn register_all(&self, hotkeys: &[HotKey]) -> Result<()> {
        self.inner.register_all(hotkeys)
    }

    pub fn unregister_all(&self, hotkeys: &[HotKey]) -> Result<()> {
        self.inner.unregister_all(hotkeys)
    }
}
