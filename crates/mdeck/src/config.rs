use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILENAME: &str = "config.yaml";
const APP_DIR: &str = "mdeck";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<DefaultsConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<RoutingWeightsConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<BTreeMap<String, String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_styles: Option<BTreeMap<String, String>>,
}

fn default_one() -> f64 {
    1.0
}

/// Configuration for routing cost weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingWeightsConfig {
    #[serde(default = "default_one")]
    pub length: f64,
    #[serde(default = "default_one")]
    pub turn: f64,
    #[serde(default = "default_one")]
    pub lane_change: f64,
    #[serde(default = "default_one")]
    pub crossing: f64,
}

impl Default for RoutingWeightsConfig {
    fn default() -> Self {
        Self {
            length: 1.0,
            turn: 1.0,
            lane_change: 1.0,
            crossing: 1.0,
        }
    }
}

impl RoutingWeightsConfig {
    /// Convert to the internal `CostWeights` type.
    pub fn to_cost_weights(&self) -> crate::render::diagram::routing::types::CostWeights {
        crate::render::diagram::routing::types::CostWeights {
            length: self.length,
            turn: self.turn,
            lane_change: self.lane_change,
            crossing: self.crossing,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_mode: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_style: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_style: Option<String>,

    /// Remembered monitor position (x, y) for fullscreen placement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor_position: Option<[f32; 2]>,
}

impl Config {
    pub fn path() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|d| d.join(APP_DIR).join(FILENAME))
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!("No config found. Run `mdeck config show` to see defaults.")
            } else {
                anyhow::anyhow!("Failed to read config: {e}")
            }
        })?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }

    pub fn save(&self) -> Result<PathBuf> {
        let path = Self::path()?;
        self.save_to(&path)?;
        Ok(path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(self)?;
        let contents = format!("# MDeck configuration — https://github.com/mklab-se/mdeck\n{yaml}");
        std::fs::write(path, contents)?;
        Ok(())
    }

    // ── Style helpers ──────────────────────────────────────────────────

    pub fn add_style(&mut self, name: &str, description: &str) {
        self.styles
            .get_or_insert_with(BTreeMap::new)
            .insert(name.to_string(), description.to_string());
    }

    pub fn remove_style(&mut self, name: &str) -> bool {
        let removed = self
            .styles
            .as_mut()
            .map(|m| m.remove(name).is_some())
            .unwrap_or(false);
        if removed {
            // Clear default if it referenced this style
            if let Some(ref defaults) = self.defaults {
                if defaults.image_style.as_deref() == Some(name) {
                    self.defaults.as_mut().unwrap().image_style = None;
                }
            }
            // Clean up empty map
            if self.styles.as_ref().is_some_and(|m| m.is_empty()) {
                self.styles = None;
            }
        }
        removed
    }

    pub fn clear_styles(&mut self) {
        self.styles = None;
        self.icon_styles = None;
        if let Some(ref mut defaults) = self.defaults {
            defaults.image_style = None;
            defaults.icon_style = None;
        }
    }

    pub fn get_style(&self, name: &str) -> Option<&str> {
        self.styles.as_ref()?.get(name).map(|s| s.as_str())
    }

    pub fn list_styles(&self) -> Vec<(&str, &str)> {
        self.styles
            .as_ref()
            .map(|m| m.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect())
            .unwrap_or_default()
    }

    pub fn add_icon_style(&mut self, name: &str, description: &str) {
        self.icon_styles
            .get_or_insert_with(BTreeMap::new)
            .insert(name.to_string(), description.to_string());
    }

    pub fn remove_icon_style(&mut self, name: &str) -> bool {
        let removed = self
            .icon_styles
            .as_mut()
            .map(|m| m.remove(name).is_some())
            .unwrap_or(false);
        if removed {
            if let Some(ref defaults) = self.defaults {
                if defaults.icon_style.as_deref() == Some(name) {
                    self.defaults.as_mut().unwrap().icon_style = None;
                }
            }
            if self.icon_styles.as_ref().is_some_and(|m| m.is_empty()) {
                self.icon_styles = None;
            }
        }
        removed
    }

    pub fn list_icon_styles(&self) -> Vec<(&str, &str)> {
        self.icon_styles
            .as_ref()
            .map(|m| m.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect())
            .unwrap_or_default()
    }

    pub fn get_icon_style(&self, name: &str) -> Option<&str> {
        self.icon_styles.as_ref()?.get(name).map(|s| s.as_str())
    }

    /// Resolve the effective image style description.
    /// Priority: defaults.image_style name → hardcoded default.
    pub fn resolve_image_style(&self) -> &str {
        if let Some(ref defaults) = self.defaults {
            if let Some(ref name) = defaults.image_style {
                if let Some(desc) = self.get_style(name) {
                    return desc;
                }
            }
        }
        crate::prompt::DEFAULT_IMAGE_STYLE
    }

    /// Resolve the effective icon style description.
    /// Priority: defaults.icon_style name → hardcoded default.
    pub fn resolve_icon_style(&self) -> &str {
        if let Some(ref defaults) = self.defaults {
            if let Some(ref name) = defaults.icon_style {
                if let Some(desc) = self.get_icon_style(name) {
                    return desc;
                }
            }
        }
        crate::prompt::DEFAULT_ICON_STYLE
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "defaults.theme" => {
                match value {
                    "light" | "dark" | "nord" => {}
                    _ => {
                        anyhow::bail!("Invalid theme: {value}. Must be 'light', 'dark', or 'nord'.")
                    }
                }
                self.defaults
                    .get_or_insert_with(DefaultsConfig::default)
                    .theme = Some(value.to_string());
            }
            "defaults.transition" => {
                match value {
                    "fade" | "slide" | "spatial" | "none" => {}
                    _ => anyhow::bail!(
                        "Invalid transition: {value}. Must be 'fade', 'slide', 'spatial', or 'none'."
                    ),
                }
                self.defaults
                    .get_or_insert_with(DefaultsConfig::default)
                    .transition = Some(value.to_string());
            }
            "defaults.aspect" => {
                match value {
                    "16:9" | "4:3" | "16:10" => {}
                    _ => anyhow::bail!(
                        "Invalid aspect ratio: {value}. Must be '16:9', '4:3', or '16:10'."
                    ),
                }
                self.defaults
                    .get_or_insert_with(DefaultsConfig::default)
                    .aspect = Some(value.to_string());
            }
            "defaults.start_mode" => {
                if value != "first" && value != "overview" && value.parse::<usize>().is_err() {
                    anyhow::bail!(
                        "Invalid start_mode: {value}. Must be 'first', 'overview', or a slide number."
                    );
                }
                self.defaults
                    .get_or_insert_with(DefaultsConfig::default)
                    .start_mode = Some(value.to_string());
            }
            "defaults.image_style" => {
                if self.get_style(value).is_none() {
                    anyhow::bail!(
                        "No image style named '{value}'. Known styles: {}. Add one with `mdeck ai style add`.",
                        Self::join_names(self.list_styles())
                    );
                }
                self.defaults
                    .get_or_insert_with(DefaultsConfig::default)
                    .image_style = Some(value.to_string());
            }
            "defaults.icon_style" => {
                if self.get_icon_style(value).is_none() {
                    anyhow::bail!(
                        "No icon style named '{value}'. Known icon styles: {}. Add one with `mdeck ai style add --icon`.",
                        Self::join_names(self.list_icon_styles())
                    );
                }
                self.defaults
                    .get_or_insert_with(DefaultsConfig::default)
                    .icon_style = Some(value.to_string());
            }
            _ => anyhow::bail!(
                "Unknown config key: {key}. Valid keys: defaults.theme, defaults.transition, defaults.aspect, defaults.start_mode, defaults.image_style, defaults.icon_style"
            ),
        }
        Ok(())
    }

    fn join_names(styles: Vec<(&str, &str)>) -> String {
        if styles.is_empty() {
            "(none)".to_string()
        } else {
            styles
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mdeck-config-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn set_valid_defaults() {
        let mut cfg = Config::default();
        cfg.set("defaults.theme", "nord").unwrap();
        cfg.set("defaults.transition", "spatial").unwrap();
        cfg.set("defaults.aspect", "4:3").unwrap();
        cfg.set("defaults.start_mode", "overview").unwrap();
        cfg.set("defaults.start_mode", "7").unwrap();
        let d = cfg.defaults.as_ref().unwrap();
        assert_eq!(d.theme.as_deref(), Some("nord"));
        assert_eq!(d.transition.as_deref(), Some("spatial"));
        assert_eq!(d.aspect.as_deref(), Some("4:3"));
        assert_eq!(d.start_mode.as_deref(), Some("7"));
    }

    #[test]
    fn set_rejects_invalid_values_and_keys() {
        let mut cfg = Config::default();
        assert!(cfg.set("defaults.theme", "solarized").is_err());
        assert!(cfg.set("defaults.transition", "zoom").is_err());
        assert!(cfg.set("defaults.aspect", "1:1").is_err());
        assert!(cfg.set("defaults.start_mode", "last").is_err());
        let err = cfg.set("defaults.nope", "x").unwrap_err().to_string();
        assert!(err.contains("Unknown config key"));
        assert!(err.contains("defaults.image_style"));
        // Nothing was written by the failed sets
        assert!(cfg.defaults.is_none());
    }

    #[test]
    fn set_image_and_icon_style_defaults() {
        let mut cfg = Config::default();
        // Unknown style names are rejected with the known names listed
        let err = cfg
            .set("defaults.image_style", "pixar")
            .unwrap_err()
            .to_string();
        assert!(err.contains("pixar"));
        assert!(err.contains("(none)"));
        assert!(cfg.set("defaults.icon_style", "flat").is_err());

        cfg.add_style("pixar", "3D animated look");
        cfg.add_icon_style("flat", "flat vector icons");
        cfg.set("defaults.image_style", "pixar").unwrap();
        cfg.set("defaults.icon_style", "flat").unwrap();
        let d = cfg.defaults.as_ref().unwrap();
        assert_eq!(d.image_style.as_deref(), Some("pixar"));
        assert_eq!(d.icon_style.as_deref(), Some("flat"));
        assert_eq!(cfg.resolve_image_style(), "3D animated look");
        assert_eq!(cfg.resolve_icon_style(), "flat vector icons");

        // Error for an unknown name lists the existing ones
        let err = cfg
            .set("defaults.image_style", "other")
            .unwrap_err()
            .to_string();
        assert!(err.contains("pixar"));

        // Removing the style clears the default
        assert!(cfg.remove_style("pixar"));
        assert!(cfg.defaults.as_ref().unwrap().image_style.is_none());
        assert_eq!(
            cfg.resolve_image_style(),
            crate::prompt::DEFAULT_IMAGE_STYLE
        );
    }

    #[test]
    fn load_missing_file_is_not_found_error() {
        let dir = temp_dir("missing");
        let err = Config::load_from(&dir.join("config.yaml")).unwrap_err();
        assert!(err.to_string().contains("No config found"));
    }

    #[test]
    fn load_bad_file_is_error() {
        let dir = temp_dir("bad");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.yaml");
        std::fs::write(&path, "defaults: [this, is, not, a, map\n").unwrap();
        assert!(Config::load_from(&path).is_err());
        // Unknown keys are tolerated; wrong types are not
        std::fs::write(&path, "defaults:\n  theme: [1, 2]\n").unwrap();
        assert!(Config::load_from(&path).is_err());
        std::fs::write(&path, "unknown_key: 1\n").unwrap();
        assert!(Config::load_from(&path).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_round_trip() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("nested").join("config.yaml");

        let mut cfg = Config::default();
        cfg.set("defaults.theme", "dark").unwrap();
        cfg.set("defaults.aspect", "16:10").unwrap();
        cfg.add_style("pixar", "3D animated look");
        cfg.add_icon_style("flat", "flat icons");
        cfg.set("defaults.image_style", "pixar").unwrap();
        cfg.routing = Some(RoutingWeightsConfig {
            turn: 2.5,
            ..Default::default()
        });
        cfg.defaults.as_mut().unwrap().monitor_position = Some([100.0, 200.0]);

        cfg.save_to(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# MDeck configuration"));

        let loaded = Config::load_from(&path).unwrap();
        let d = loaded.defaults.as_ref().unwrap();
        assert_eq!(d.theme.as_deref(), Some("dark"));
        assert_eq!(d.aspect.as_deref(), Some("16:10"));
        assert_eq!(d.image_style.as_deref(), Some("pixar"));
        assert_eq!(d.monitor_position, Some([100.0, 200.0]));
        assert!(d.transition.is_none());
        assert_eq!(loaded.get_style("pixar"), Some("3D animated look"));
        assert_eq!(loaded.get_icon_style("flat"), Some("flat icons"));
        assert_eq!(loaded.routing.as_ref().unwrap().turn, 2.5);
        assert_eq!(loaded.routing.as_ref().unwrap().length, 1.0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_config_serializes_minimal() {
        let dir = temp_dir("empty");
        let path = dir.join("config.yaml");
        Config::default().save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert!(loaded.defaults.is_none());
        assert!(loaded.styles.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
