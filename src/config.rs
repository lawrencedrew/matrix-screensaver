use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Charset {
    Katakana,
    Latin,
    Digits,
    Mixed,
}

impl Default for Charset {
    fn default() -> Self {
        Charset::Katakana
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_timeout")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default)]
    pub charset: Charset,
}

fn default_timeout() -> u64 { 300 }
fn default_color() -> String { "#00ff00".to_string() }
fn default_fps() -> u32 { 30 }
fn default_speed() -> f32 { 1.0 }

impl Default for Config {
    fn default() -> Self {
        Config {
            idle_timeout_secs: default_timeout(),
            color: default_color(),
            fps: default_fps(),
            speed: default_speed(),
            charset: Charset::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        config_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
}

fn config_path() -> Option<PathBuf> {
    let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else {
        let home = std::env::var("HOME").ok()?;
        PathBuf::from(home).join(".config")
    };
    Some(base.join("matrix-screensaver").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.idle_timeout_secs, 300);
        assert_eq!(cfg.fps, 30);
        assert!((cfg.speed - 1.0).abs() < f32::EPSILON);
        assert_eq!(cfg.charset, Charset::Katakana);
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
            idle_timeout_secs = 60
            fps = 20
            speed = 2.0
            charset = "mixed"
        "#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.idle_timeout_secs, 60);
        assert_eq!(cfg.fps, 20);
        assert!((cfg.speed - 2.0).abs() < f32::EPSILON);
        assert_eq!(cfg.charset, Charset::Mixed);
    }
}
