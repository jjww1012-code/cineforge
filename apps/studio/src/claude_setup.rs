//! Ein-Tasten-Einrichtung: trägt CineForge in die Konfiguration von
//! Claude Desktop ein (F2). Vorhandene Einträge bleiben erhalten, vorher wird
//! eine Sicherheitskopie angelegt.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde_json::{json, Value};

pub struct ClaudeSetupPlugin;

impl Plugin for ClaudeSetupPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SetupState::detect()).add_systems(Update, setup_key);
    }
}

#[derive(Resource, Debug, Clone)]
pub struct SetupState {
    /// Kurzer Text für die Anzeige.
    pub message: String,
}

impl SetupState {
    fn detect() -> Self {
        let message = match config_path() {
            None => "Claude Desktop: nicht gefunden".into(),
            Some(p) if is_configured(&p) => "Claude Desktop: verbunden ✓".into(),
            Some(_) => "Claude Desktop: F2 drücken zum Verbinden".into(),
        };
        Self { message }
    }
}

/// Pfad der Claude-Desktop-Konfiguration, falls Claude Desktop installiert ist.
pub fn config_path() -> Option<PathBuf> {
    let dir = if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/Claude")
    } else if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?).join("Claude")
    } else {
        PathBuf::from(std::env::var_os("HOME")?).join(".config/Claude")
    };
    dir.is_dir().then(|| dir.join("claude_desktop_config.json"))
}

fn is_configured(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .is_some_and(|v| v["mcpServers"]["cineforge"].is_object())
}

/// Fügt den CineForge-Eintrag in eine bestehende Konfiguration ein.
pub fn merge_config(existing: Option<&str>, exe: &str) -> Result<String, String> {
    let mut v: Value = match existing.map(str::trim) {
        None | Some("") => json!({}),
        Some(t) => serde_json::from_str(t)
            .map_err(|e| format!("Bestehende Claude-Konfiguration ist kein gültiges JSON ({e}) – bitte prüfen"))?,
    };
    let root = v.as_object_mut().ok_or("Claude-Konfiguration hat unerwartetes Format")?;
    let servers = root.entry("mcpServers").or_insert_with(|| json!({}));
    let servers = servers.as_object_mut().ok_or("mcpServers hat unerwartetes Format")?;
    servers.insert("cineforge".into(), json!({"command": exe, "args": ["mcp"]}));
    Ok(serde_json::to_string_pretty(&v).expect("serialisierbar"))
}

fn install() -> Result<String, String> {
    let path = config_path().ok_or("Claude Desktop ist nicht installiert (claude.ai/download)")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.to_string_lossy().to_string();
    // macOS startet Apps aus „Downloads“ an einem zufälligen Ort (App Translocation).
    // Dieser Pfad ändert sich bei jedem Start, darauf darf die Konfiguration nicht zeigen.
    if exe_str.contains("/AppTranslocation/") {
        return Err("Bitte CineForge zuerst in den Programme-Ordner ziehen und von dort starten.".into());
    }
    let existing = std::fs::read_to_string(&path).ok();
    let merged = merge_config(existing.as_deref(), &exe_str)?;
    if let Some(old) = &existing {
        let _ = std::fs::write(path.with_extension("json.cineforge-backup"), old);
    }
    std::fs::write(&path, merged).map_err(|e| format!("Konnte {} nicht schreiben: {e}", path.display()))?;
    Ok("Claude Desktop verbunden ✓ – bitte Claude Desktop einmal komplett beenden und neu öffnen.".into())
}

fn setup_key(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<SetupState>) {
    if keys.just_pressed(KeyCode::F2) {
        state.message = match install() {
            Ok(m) => m,
            Err(e) => format!("Claude Desktop: {e}"),
        };
        info!("{}", state.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_keeps_other_servers_and_settings() {
        let existing = r#"{"mcpServers":{"other":{"command":"x"}},"theme":"dark"}"#;
        let out: Value = serde_json::from_str(&merge_config(Some(existing), "/A/cineforge").unwrap()).unwrap();
        assert_eq!(out["mcpServers"]["other"]["command"], "x");
        assert_eq!(out["theme"], "dark");
        assert_eq!(out["mcpServers"]["cineforge"]["args"][0], "mcp");
        assert_eq!(out["mcpServers"]["cineforge"]["command"], "/A/cineforge");
    }

    #[test]
    fn merge_creates_fresh_config_and_rejects_garbage() {
        let out: Value = serde_json::from_str(&merge_config(None, "c").unwrap()).unwrap();
        assert!(out["mcpServers"]["cineforge"].is_object());
        assert!(merge_config(Some("{kaputt"), "c").is_err());
        assert!(merge_config(Some("[]"), "c").is_err());
    }
}
