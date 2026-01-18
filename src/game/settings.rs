
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub window_position: Option<(i32, i32)>,
    pub window_size: Option<(u32, u32)>,
    pub fullscreen: bool,
    pub volume: f32,
    pub music_volume: f32,
    pub muted: bool,

    // Non-persistent settings can be added here
    #[serde(skip)]
    pub dirty: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            window_position: None,
            window_size: None,
            fullscreen: false,
            volume: 1.0,
            music_volume: 1.0,
            muted: false,
            dirty: false,
        }
    }
}

impl GameSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Self {
        let settings_path = get_settings_path();
        match Self::load_from_file(settings_path.to_str().unwrap_or("settings.toml")) {
            Ok(settings) => settings,
            Err(_) => Self::default(),
        }
    }

    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.dirty {
            let settings_path = get_settings_path();
            if let Some(parent) = settings_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            self.save_to_file(settings_path.to_str().unwrap_or("settings.toml"))?;
        }
        Ok(())
    }

    fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)?;
        let settings: GameSettings = toml::from_str(&data)?;
        Ok(settings)
    }

    fn save_to_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let data = toml::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        self.dirty = false;
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f32) {
        if self.volume != volume {
            self.volume = volume.clamp(0.0, 1.0);
            self.dirty = true;
        }
    }

    pub fn set_music_volume(&mut self, music_volume: f32) {
        if self.music_volume != music_volume {
            self.music_volume = music_volume.clamp(0.0, 1.0);
            self.dirty = true;
        }
    }

    pub fn set_muted(&mut self, muted: bool) {
        if self.muted != muted {
            self.muted = muted;
            self.dirty = true;
        }
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if self.fullscreen != fullscreen {
            self.fullscreen = fullscreen;
            self.dirty = true;
        }
    }

    pub fn set_window_size(&mut self, size: (u32, u32)) {
        if self.window_size != Some(size) {
            self.window_size = Some(size);
            self.dirty = true;
        }
    }

    pub fn set_window_position(&mut self, position: (i32, i32)) {
        if self.window_position != Some(position) {
            self.window_position = Some(position);
            self.dirty = true;
        }
    }
}

fn get_settings_path() -> std::path::PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    config_dir.join("faery").join("settings.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_path() {
        let path = get_settings_path();
        assert!(path.ends_with("faery/settings.toml"));
    }

    #[test]
    fn test_volume_settings() {
        let mut settings = GameSettings::new();

        settings.set_volume(0.5);
        assert_eq!(settings.volume, 0.5);
        assert!(settings.dirty);
        settings.dirty = false; // reset dirty flag

        settings.set_music_volume(0.8);
        assert_eq!(settings.music_volume, 0.8);
        assert!(settings.dirty);
        settings.dirty = false; // reset dirty flag

        settings.set_muted(true);
        assert!(settings.muted);
        assert!(settings.dirty);
    }

    #[test]
    fn test_window_frame() {
        let mut settings = GameSettings::new();
        let rect = sdl2::rect::Rect::new(100, 100, 800, 600);
        settings.set_window_frame(rect);
        assert_eq!(settings.get_window_frame(), Some(rect));
        assert!(settings.dirty);
        settings.dirty = false; // reset dirty flag

        settings.set_fullscreen(true);
        assert!(settings.fullscreen);
        assert!(settings.dirty);
    }
}
