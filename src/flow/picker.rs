//! Keyboard-only settings, kept as a draft until Save. Nothing reaches the
//! assistant through local keyboard input while the picker is open. Remote
//! dashboard input still targets the assistant, whose screen it displays.

use super::scene;
use crate::config::FlowPreferences;
use crossterm::event::{KeyCode, KeyEvent};

pub const COLORS: &[(&str, Option<f64>)] = &[
    ("session", None),
    ("natural", None),
    ("rose", Some(0.0)),
    ("crimson", Some(25.0)),
    ("ember", Some(50.0)),
    ("amber", Some(70.0)),
    ("gold", Some(95.0)),
    ("lime", Some(125.0)),
    ("jade", Some(155.0)),
    ("teal", Some(185.0)),
    ("cyan", Some(210.0)),
    ("azure", Some(240.0)),
    ("blue", Some(265.0)),
    ("indigo", Some(285.0)),
    ("violet", Some(305.0)),
    ("magenta", Some(335.0)),
];

pub struct Picker {
    pub draft: FlowPreferences,
    row: usize,
    scenes: bool,
    pub error: bool,
}

pub enum Action {
    Edit,
    Cancel,
    Save(FlowPreferences),
}

impl Picker {
    pub fn new(draft: FlowPreferences) -> Self {
        Self {
            draft,
            row: 0,
            scenes: false,
            error: false,
        }
    }

    fn names() -> Vec<&'static str> {
        std::iter::once("random")
            .chain(scene::scenes().iter().map(|s| s.name))
            .collect()
    }

    pub fn key(&mut self, key: KeyEvent) -> Action {
        let count = if self.scenes { Self::names().len() } else { 7 };
        match key.code {
            KeyCode::Esc if self.scenes => {
                self.scenes = false;
                self.row = 0;
            }
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Up => self.row = (self.row + count - 1) % count,
            KeyCode::Down | KeyCode::Tab => self.row = (self.row + 1) % count,
            KeyCode::Enter if self.scenes => {
                self.draft.scene = Self::names()[self.row].to_string();
                self.scenes = false;
                self.row = 0;
            }
            KeyCode::Enter if self.row == 0 => {
                self.scenes = true;
                self.row = Self::names()
                    .iter()
                    .position(|n| *n == self.draft.scene)
                    .unwrap_or(0);
            }
            KeyCode::Enter => return Action::Save(self.draft.clone()),
            KeyCode::Left | KeyCode::Right if !self.scenes => {
                let forward = key.code == KeyCode::Right;
                let rotate = |at: usize, len: usize| {
                    if forward {
                        (at + 1) % len
                    } else {
                        (at + len - 1) % len
                    }
                };
                match self.row {
                    0 => {
                        let names = Self::names();
                        let at = names
                            .iter()
                            .position(|n| *n == self.draft.scene)
                            .unwrap_or(0);
                        self.draft.scene = names[rotate(at, names.len())].to_string();
                    }
                    1 => self.draft.enabled = !self.draft.enabled,
                    2 => self.draft.fade = !self.draft.fade,
                    3 => {
                        let at = self.draft.level.map_or(0, |n| usize::from(n.min(10)) + 1);
                        self.draft.level = rotate(at, 12).checked_sub(1).map(|n| n as u8);
                    }
                    4 => {
                        let at = match self.draft.night {
                            None => 0,
                            Some(false) => 1,
                            Some(true) => 2,
                        };
                        self.draft.night = [None, Some(false), Some(true)][rotate(at, 3)];
                    }
                    5 => {
                        let at = COLORS
                            .iter()
                            .position(|(n, _)| *n == self.draft.color)
                            .unwrap_or(0);
                        self.draft.color = COLORS[rotate(at, COLORS.len())].0.to_string();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Action::Edit
    }

    pub fn rows(&self, width: usize, height: usize) -> Vec<String> {
        let labels = if self.scenes {
            Self::names()
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
        } else {
            vec![
                self.draft.scene.clone(),
                format!("Show: {}", if self.draft.enabled { "on" } else { "off" }),
                format!("Fade: {}", if self.draft.fade { "on" } else { "off" }),
                format!(
                    "Level: {}",
                    self.draft.level.map_or("auto".into(), |n| n.to_string())
                ),
                format!(
                    "Time: {}",
                    match self.draft.night {
                        None => "auto",
                        Some(true) => "night",
                        Some(false) => "day",
                    }
                ),
                format!("Hue: {}", self.draft.color),
                if self.error {
                    "Save failed".into()
                } else {
                    "Save".into()
                },
            ]
        };
        let visible = height.saturating_sub(1).max(1);
        let top = self.row.saturating_sub(visible - 1);
        let mut rows: Vec<String> = (0..visible)
            .map(|i| {
                let index = top + i;
                labels.get(index).map_or(String::new(), |label| {
                    format!("{}{}", if index == self.row { "› " } else { "  " }, label)
                })
            })
            .collect();
        rows.push(
            if self.error {
                "Save failed"
            } else if self.scenes {
                "Enter pick Esc"
            } else {
                "←→ edit ↵ save"
            }
            .into(),
        );
        rows.truncate(height);
        rows.into_iter()
            .map(|s| crate::ui::utils::fit_ansi_to_width(&s, width))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn scene_list_reaches_every_scene_and_the_short_column_scrolls() {
        let mut picker = Picker::new(FlowPreferences::default());
        picker.key(key(KeyCode::Enter));
        for name in Picker::names() {
            assert!(picker.rows(15, 4).iter().any(|r| r.contains(name)));
            picker.key(key(KeyCode::Down));
        }
        picker.key(key(KeyCode::Down));
        picker.key(key(KeyCode::Enter));
        assert_eq!(picker.draft.scene, "fire");
        picker.key(key(KeyCode::Down));
        assert!(matches!(picker.key(key(KeyCode::Enter)), Action::Save(_)));
    }

    #[test]
    fn edits_cover_auto_and_manual_settings_without_saving_on_navigation() {
        let mut picker = Picker::new(FlowPreferences::default());
        picker.row = 3;
        picker.key(key(KeyCode::Left));
        assert_eq!(picker.draft.level, Some(10));
        picker.key(key(KeyCode::Right));
        assert_eq!(picker.draft.level, None);
        picker.row = 4;
        picker.key(key(KeyCode::Left));
        assert_eq!(picker.draft.night, Some(true));
        picker.row = 5;
        picker.key(key(KeyCode::Right));
        assert_eq!(picker.draft.color, "natural");
        assert!(matches!(picker.key(key(KeyCode::Esc)), Action::Cancel));
    }
}
