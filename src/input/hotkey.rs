use anyhow::{anyhow, Context, Result};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

use crate::config::HotkeyConfig;

/// Application state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Idle,
    Recording,
    Processing,
}

/// Global hotkey manager with state tracking
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
    state: Arc<Mutex<AppState>>,
}

impl HotkeyManager {
    /// Create and register global hotkey from config
    pub fn new(config: &HotkeyConfig) -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("failed to create hotkey manager")?;

        let modifiers = Self::parse_modifiers(&config.modifiers)?;
        let code = Self::parse_key(&config.key)?;

        let hotkey = HotKey::new(Some(modifiers), code);
        manager
            .register(hotkey)
            .context("failed to register hotkey")?;

        info!("registered hotkey: {:?} + {}", config.modifiers, config.key);

        Ok(Self {
            manager,
            hotkey,
            state: Arc::new(Mutex::new(AppState::Idle)),
        })
    }

    /// Get current state
    pub fn state(&self) -> AppState {
        *self.state.lock().unwrap()
    }

    /// Handle hotkey press event
    pub fn on_press(&self) {
        let mut state = self.state.lock().unwrap();
        match *state {
            AppState::Idle => {
                info!("hotkey pressed: Idle → Recording");
                *state = AppState::Recording;
                // Phase 3: Start audio recording
            }
            AppState::Recording => {
                debug!("hotkey pressed while recording (ignored)");
            }
            AppState::Processing => {
                debug!("hotkey pressed while processing (ignored)");
            }
        }
    }

    /// Handle hotkey release event
    pub fn on_release(&self) {
        let mut state = self.state.lock().unwrap();
        match *state {
            AppState::Recording => {
                info!("hotkey released: Recording → Processing");
                *state = AppState::Processing;
                // Phase 3: Stop audio recording
                // Phase 4: Send to transcription
                // For now, immediately return to Idle (no audio yet)
                *state = AppState::Idle;
                info!("processing complete: Processing → Idle");
            }
            AppState::Idle => {
                debug!("hotkey released while idle (ignored)");
            }
            AppState::Processing => {
                debug!("hotkey released while processing (ignored)");
            }
        }
    }

    /// Process hotkey events from global event channel
    pub fn handle_event(&self, event: GlobalHotKeyEvent) {
        if event.id != self.hotkey.id() {
            return;
        }

        match event.state {
            global_hotkey::HotKeyState::Pressed => self.on_press(),
            global_hotkey::HotKeyState::Released => self.on_release(),
        }
    }

    fn parse_modifiers(modifiers: &[String]) -> Result<Modifiers> {
        let mut result = Modifiers::empty();
        for modifier in modifiers {
            match modifier.as_str() {
                "Control" | "Ctrl" => result |= Modifiers::CONTROL,
                "Option" | "Alt" => result |= Modifiers::ALT,
                "Command" | "Super" => result |= Modifiers::SUPER,
                "Shift" => result |= Modifiers::SHIFT,
                _ => return Err(anyhow!("unknown modifier: {}", modifier)),
            }
        }
        Ok(result)
    }

    fn parse_key(key: &str) -> Result<Code> {
        match key {
            "A" => Ok(Code::KeyA),
            "B" => Ok(Code::KeyB),
            "C" => Ok(Code::KeyC),
            "D" => Ok(Code::KeyD),
            "E" => Ok(Code::KeyE),
            "F" => Ok(Code::KeyF),
            "G" => Ok(Code::KeyG),
            "H" => Ok(Code::KeyH),
            "I" => Ok(Code::KeyI),
            "J" => Ok(Code::KeyJ),
            "K" => Ok(Code::KeyK),
            "L" => Ok(Code::KeyL),
            "M" => Ok(Code::KeyM),
            "N" => Ok(Code::KeyN),
            "O" => Ok(Code::KeyO),
            "P" => Ok(Code::KeyP),
            "Q" => Ok(Code::KeyQ),
            "R" => Ok(Code::KeyR),
            "S" => Ok(Code::KeyS),
            "T" => Ok(Code::KeyT),
            "U" => Ok(Code::KeyU),
            "V" => Ok(Code::KeyV),
            "W" => Ok(Code::KeyW),
            "X" => Ok(Code::KeyX),
            "Y" => Ok(Code::KeyY),
            "Z" => Ok(Code::KeyZ),
            _ => Err(anyhow!("unsupported key: {}", key)),
        }
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        if let Err(e) = self.manager.unregister(self.hotkey) {
            tracing::error!("failed to unregister hotkey: {}", e);
        }
    }
}
