use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tauri::{AppHandle, Emitter};
use wayclip_global_hotkey::{
    hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};

#[derive(Clone)]
pub struct LinuxShortcutState {
    manager: Arc<RwLock<Option<GlobalHotKeyManager>>>,
    registered: Arc<RwLock<Vec<HotKey>>>,
    actions: Arc<RwLock<HashMap<u32, String>>>,
    init_error: Arc<RwLock<Option<String>>>,
}

impl LinuxShortcutState {
    pub fn new(app: &AppHandle) -> Self {
        let manager = GlobalHotKeyManager::new();
        let (manager, init_error) = match manager {
            Ok(manager) => (Some(manager), None),
            Err(error) => {
                let detail = format!("global shortcut backend unavailable: {error}");
                eprintln!("[shortcuts] {detail}");
                (None, Some(detail))
            }
        };

        let state = Self {
            manager: Arc::new(RwLock::new(manager)),
            registered: Arc::new(RwLock::new(Vec::new())),
            actions: Arc::new(RwLock::new(HashMap::new())),
            init_error: Arc::new(RwLock::new(init_error)),
        };
        state.start_event_forwarder(app.clone());
        state
    }

    fn start_event_forwarder(&self, app: AppHandle) {
        let actions = self.actions.clone();
        std::thread::spawn(move || loop {
            match GlobalHotKeyEvent::receiver().recv() {
                Ok(event) if event.state == HotKeyState::Pressed => {
                    let action = actions
                        .read()
                        .ok()
                        .and_then(|bindings| bindings.get(&event.id).cloned());
                    if let Some(action) = action {
                        let _ = app.emit("shortcut-action", action);
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        });
    }

    pub fn register(&self, bindings: &[(String, String)]) -> Result<usize, String> {
        if let Some(error) = self
            .init_error
            .read()
            .map_err(|error| error.to_string())?
            .clone()
        {
            return Err(error);
        }

        let parsed: Result<Vec<(HotKey, String)>, String> = bindings
            .iter()
            .map(|(action, shortcut)| {
                let hotkey = shortcut
                    .parse::<HotKey>()
                    .map_err(|error| format!("invalid shortcut {shortcut}: {error}"))?;
                Ok((hotkey, action.clone()))
            })
            .collect();
        let parsed = parsed?;

        let manager_guard = self.manager.read().map_err(|error| error.to_string())?;
        let manager = manager_guard
            .as_ref()
            .ok_or_else(|| "global shortcut backend unavailable".to_string())?;

        let previous = self
            .registered
            .read()
            .map_err(|error| error.to_string())?
            .clone();
        if !previous.is_empty() {
            manager
                .unregister_all(&previous)
                .map_err(|error| format!("failed to unregister previous shortcuts: {error}"))?;
        }

        let hotkeys: Vec<HotKey> = parsed.iter().map(|(hotkey, _)| *hotkey).collect();
        if let Err(error) = manager.register_all(&hotkeys) {
            self.registered
                .write()
                .map_err(|lock_error| lock_error.to_string())?
                .clear();
            self.actions
                .write()
                .map_err(|lock_error| lock_error.to_string())?
                .clear();
            return Err(format!("failed to register global shortcuts: {error}"));
        }

        let actions = parsed
            .iter()
            .map(|(hotkey, action)| (hotkey.id(), action.clone()))
            .collect();
        *self.actions.write().map_err(|error| error.to_string())? = actions;
        *self.registered.write().map_err(|error| error.to_string())? = hotkeys;
        Ok(parsed.len())
    }

    pub fn unregister_all(&self) -> Result<(), String> {
        let manager_guard = self.manager.read().map_err(|error| error.to_string())?;
        if let Some(manager) = manager_guard.as_ref() {
            let previous = self
                .registered
                .read()
                .map_err(|error| error.to_string())?
                .clone();
            if !previous.is_empty() {
                manager
                    .unregister_all(&previous)
                    .map_err(|error| format!("failed to unregister shortcuts: {error}"))?;
            }
        }
        self.registered
            .write()
            .map_err(|error| error.to_string())?
            .clear();
        self.actions
            .write()
            .map_err(|error| error.to_string())?
            .clear();
        Ok(())
    }
}
