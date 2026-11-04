//! Projekt-Logbuch: das Gedächtnis eines Film-Projekts.
//!
//! Hier steht alles, was nötig ist, damit Claude (oder ein Mensch) nach einer
//! Unterbrechung – z. B. wenn das Nutzungslimit erreicht ist – genau dort
//! weitermachen kann, wo aufgehört wurde: Ziel, Stil, Storyboard, Aufgaben,
//! Notizen, Protokoll und der nächste Schritt.
//!
//! Die Datei wird nach jeder Änderung sofort gespeichert.

use serde::{Deserialize, Serialize};

pub const PROJECT_FILE: &str = "projekt.json";
const MAX_LOG: usize = 400;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Project {
    pub version: u32,
    pub name: String,
    /// Idee / Ziel des Projekts in Klartext.
    pub goal: String,
    /// Stil-Vorgaben (Look, Farben, Stimmung, Tempo), damit alle Szenen zusammenpassen.
    pub style: String,
    /// Reihenfolge der Einstellungen (Shots) des Films.
    pub storyboard: Vec<Shot>,
    pub tasks: Vec<Task>,
    /// Wichtige Entscheidungen und Erkenntnisse.
    pub notes: Vec<Note>,
    /// Übergabe-Notiz: Was ist als Nächstes zu tun?
    pub next_step: String,
    /// Automatisches Protokoll (neueste zuletzt, begrenzt).
    pub log: Vec<LogEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Open,
    InProgress,
    Done,
    Blocked,
}

impl Status {
    pub fn symbol(self) -> &'static str {
        match self {
            Status::Open => "☐",
            Status::InProgress => "◐",
            Status::Done => "☑",
            Status::Blocked => "⛔",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Shot {
    /// Stabile Kennung, z. B. "s01_ankunft".
    pub id: String,
    pub title: String,
    /// Was passiert, was sieht und hört man?
    pub description: String,
    /// Name der Szenen-Datei (ohne .json) im Projekt.
    pub scene: Option<String>,
    /// Dauer in Sekunden.
    pub duration: f32,
    pub status: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Task {
    pub id: u32,
    pub text: String,
    pub status: Status,
    pub notes: String,
    pub updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Note {
    pub time: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct LogEntry {
    pub time: String,
    /// "claude" oder "mensch"
    pub actor: String,
    pub action: String,
}

impl Project {
    pub fn new(name: &str, goal: &str) -> Self {
        Self { version: 1, name: name.into(), goal: goal.into(), ..Default::default() }
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("Projekt-Datei nicht lesbar: {e}"))
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialisierbar")
    }

    pub fn log(&mut self, time: &str, actor: &str, action: impl Into<String>) {
        self.log.push(LogEntry { time: time.into(), actor: actor.into(), action: action.into() });
        if self.log.len() > MAX_LOG {
            let excess = self.log.len() - MAX_LOG;
            self.log.drain(..excess);
        }
    }

    /// Storyboard ändern: Shots mit bekannter id werden zusammengeführt
    /// (nur angegebene Felder), neue werden angehängt (oder an `position`
    /// eingefügt), `remove: true` löscht.
    pub fn update_storyboard(&mut self, changes: &[serde_json::Value]) -> Result<Vec<String>, String> {
        let mut report = Vec::new();
        for ch in changes {
            let id = ch
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("Jeder Shot braucht eine \"id\"")?
                .to_string();
            if ch.get("remove").and_then(|v| v.as_bool()) == Some(true) {
                let before = self.storyboard.len();
                self.storyboard.retain(|s| s.id != id);
                if before == self.storyboard.len() {
                    return Err(format!("Shot \"{id}\" gibt es nicht"));
                }
                report.push(format!("Shot {id} entfernt"));
                continue;
            }
            let mut patch = ch.clone();
            if let Some(m) = patch.as_object_mut() {
                m.remove("position");
            }
            match self.storyboard.iter().position(|s| s.id == id) {
                Some(i) => {
                    let mut v = serde_json::to_value(&self.storyboard[i]).expect("serialisierbar");
                    crate::ops::merge_patch(&mut v, &patch);
                    self.storyboard[i] = serde_json::from_value(v).map_err(|e| format!("Shot \"{id}\": {e}"))?;
                    report.push(format!("Shot {id} geändert"));
                }
                None => {
                    let shot: Shot = serde_json::from_value(patch).map_err(|e| format!("Shot \"{id}\": {e}"))?;
                    let pos = ch
                        .get("position")
                        .and_then(|v| v.as_u64())
                        .map(|p| (p as usize).min(self.storyboard.len()))
                        .unwrap_or(self.storyboard.len());
                    self.storyboard.insert(pos, shot);
                    report.push(format!("Shot {id} hinzugefügt"));
                }
            }
        }
        Ok(report)
    }

    pub fn add_task(&mut self, text: &str, time: &str) -> u32 {
        let id = self.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.tasks.push(Task { id, text: text.into(), status: Status::Open, notes: String::new(), updated: time.into() });
        id
    }

    pub fn update_task(
        &mut self,
        id: u32,
        status: Option<Status>,
        text: Option<&str>,
        notes: Option<&str>,
        time: &str,
    ) -> Result<(), String> {
        let t = self.tasks.iter_mut().find(|t| t.id == id).ok_or_else(|| format!("Aufgabe #{id} gibt es nicht"))?;
        if let Some(s) = status {
            t.status = s;
        }
        if let Some(x) = text {
            t.text = x.into();
        }
        if let Some(n) = notes {
            t.notes = n.into();
        }
        t.updated = time.into();
        Ok(())
    }

    /// Zusammenfassung zum Weitermachen – das Erste, was Claude in einer neuen
    /// Unterhaltung liest.
    pub fn digest(&self, current_scene: Option<&str>) -> String {
        let mut s = format!("# Projekt: {}\n\n", self.name);
        s += &format!("## Ziel\n{}\n\n", or_dash(&self.goal));
        s += &format!("## Stil\n{}\n\n", or_dash(&self.style));
        s += &format!("## Nächster Schritt (Übergabe-Notiz)\n{}\n\n", or_dash(&self.next_step));

        if !self.storyboard.is_empty() {
            let total: f32 = self.storyboard.iter().map(|x| x.duration).sum();
            let done = self.storyboard.iter().filter(|x| x.status == Status::Done).count();
            s += &format!(
                "## Storyboard ({} Shots, {:.0} s gesamt, {done} fertig)\n",
                self.storyboard.len(),
                total
            );
            for (i, sh) in self.storyboard.iter().enumerate() {
                s += &format!(
                    "{} {:>2}. {} – {} ({:.1} s){}\n     {}\n",
                    sh.status.symbol(),
                    i + 1,
                    sh.id,
                    sh.title,
                    sh.duration,
                    sh.scene.as_ref().map(|x| format!(" → Szene \"{x}\"")).unwrap_or_default(),
                    sh.description
                );
            }
            s.push('\n');
        }

        let open: Vec<_> = self.tasks.iter().filter(|t| t.status != Status::Done).collect();
        let done = self.tasks.len() - open.len();
        s += &format!("## Aufgaben ({} offen, {done} erledigt)\n", open.len());
        if open.is_empty() {
            s += "(keine offenen Aufgaben)\n";
        }
        for t in &open {
            s += &format!("{} #{} {}", t.status.symbol(), t.id, t.text);
            if !t.notes.is_empty() {
                s += &format!(" — {}", t.notes);
            }
            s.push('\n');
        }
        let recent_done: Vec<_> = self.tasks.iter().filter(|t| t.status == Status::Done).rev().take(5).collect();
        if !recent_done.is_empty() {
            s += "Zuletzt erledigt:\n";
            for t in recent_done.iter().rev() {
                s += &format!("☑ #{} {}\n", t.id, t.text);
            }
        }
        s.push('\n');

        if !self.notes.is_empty() {
            s += "## Notizen (neueste zuletzt)\n";
            for n in self.notes.iter().rev().take(15).collect::<Vec<_>>().into_iter().rev() {
                s += &format!("- [{}] {}\n", n.time, n.text);
            }
            s.push('\n');
        }

        if !self.log.is_empty() {
            s += "## Letzte Aktionen\n";
            for l in self.log.iter().rev().take(20).collect::<Vec<_>>().into_iter().rev() {
                s += &format!("- [{}] {}: {}\n", l.time, l.actor, l.action);
            }
            s.push('\n');
        }
        if let Some(sc) = current_scene {
            s += &format!("## Geöffnete Szene\n{sc}\n");
        }
        s
    }
}

fn or_dash(s: &str) -> &str {
    if s.trim().is_empty() { "–" } else { s }
}

pub fn parse_status(s: &str) -> Result<Status, String> {
    match s {
        "open" | "offen" => Ok(Status::Open),
        "in_progress" | "in_arbeit" => Ok(Status::InProgress),
        "done" | "erledigt" | "fertig" => Ok(Status::Done),
        "blocked" | "blockiert" => Ok(Status::Blocked),
        _ => Err(format!("Unbekannter Status \"{s}\" (open, in_progress, done, blocked)")),
    }
}

/// Erlaubt nur einfache Namen (für Datei- und Ordnernamen).
pub fn safe_name(name: &str) -> Result<String, String> {
    let n = name.trim().trim_end_matches(".json").trim();
    let valid = !n.is_empty()
        && n.len() <= 80
        && !n.starts_with('.')
        && n.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '.'));
    if valid {
        Ok(n.to_string())
    } else {
        Err(format!("Ungültiger Name \"{name}\" (erlaubt: Buchstaben, Zahlen, - _ . Leerzeichen; max. 80 Zeichen)"))
    }
}

/// UTC-Zeitstempel "2026-11-04 18:30" ohne externe Abhängigkeit.
pub fn format_utc(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let rem = unix_secs % 86_400;
    // Algorithmus "civil_from_days" (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02} UTC", rem / 3600, (rem % 3600) / 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn utc_format() {
        assert_eq!(format_utc(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_utc(1_793_817_119), "2026-11-04 18:31 UTC");
    }

    #[test]
    fn storyboard_merge_insert_remove() {
        let mut p = Project::new("Film", "Test");
        p.update_storyboard(&[
            json!({"id": "a", "title": "Anfang", "duration": 5}),
            json!({"id": "c", "title": "Ende", "duration": 3}),
        ])
        .unwrap();
        p.update_storyboard(&[json!({"id": "b", "title": "Mitte", "position": 1})]).unwrap();
        assert_eq!(p.storyboard.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
        p.update_storyboard(&[json!({"id": "a", "status": "done", "scene": "anfang"})]).unwrap();
        assert_eq!(p.storyboard[0].status, Status::Done);
        assert_eq!(p.storyboard[0].title, "Anfang", "nicht genannte Felder bleiben");
        p.update_storyboard(&[json!({"id": "c", "remove": true})]).unwrap();
        assert_eq!(p.storyboard.len(), 2);
        assert!(p.update_storyboard(&[json!({"id": "x", "remove": true})]).is_err());
        assert!(p.update_storyboard(&[json!({"title": "ohne id"})]).is_err());
        assert!(p.update_storyboard(&[json!({"id": "a", "status": "quatsch"})]).is_err());
    }

    #[test]
    fn tasks_and_digest() {
        let mut p = Project::new("Raumstation", "10-Minuten-Sci-Fi");
        let a = p.add_task("Raumstation modellieren", "t1");
        let b = p.add_task("Shot 1 bauen", "t1");
        p.update_task(a, Some(Status::Done), None, None, "t2").unwrap();
        p.update_task(b, Some(Status::InProgress), None, Some("Kamera fehlt noch"), "t2").unwrap();
        assert!(p.update_task(99, None, None, None, "t").is_err());
        p.next_step = "Kamerafahrt für Shot 1".into();
        p.log("t3", "claude", "update_object station");
        let d = p.digest(Some("Szene: x"));
        assert!(d.contains("10-Minuten-Sci-Fi"));
        assert!(d.contains("◐ #2 Shot 1 bauen — Kamera fehlt noch"), "{d}");
        assert!(d.contains("☑ #1 Raumstation modellieren"));
        assert!(d.contains("Kamerafahrt für Shot 1"));
        assert!(d.contains("claude: update_object station"));
        let again = Project::from_json(&p.to_json_pretty()).unwrap();
        assert_eq!(p, again);
    }

    #[test]
    fn log_is_capped() {
        let mut p = Project::default();
        for i in 0..(MAX_LOG + 50) {
            p.log("t", "claude", format!("a{i}"));
        }
        assert_eq!(p.log.len(), MAX_LOG);
        assert_eq!(p.log.last().unwrap().action, format!("a{}", MAX_LOG + 49));
    }

    #[test]
    fn names_are_sanitized() {
        assert_eq!(safe_name(" Mein Film.json ").unwrap(), "Mein Film");
        assert!(safe_name("../etc").is_err());
        assert!(safe_name("a/b").is_err());
        assert!(safe_name("").is_err());
        assert!(safe_name(".versteckt").is_err());
    }
}
