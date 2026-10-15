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
            error: false,
        }
    }

    fn toggle_scene(&mut self) {
        let name = scene::scenes()[self.row].name;
        if self.draft.disabled_scenes.iter().any(|n| n == name) {
            self.draft.disabled_scenes.retain(|n| n != name);
        } else {
            self.draft.disabled_scenes.push(name.into());
        }
    }

    pub fn key(&mut self, key: KeyEvent) -> Action {
        let scenes = scene::scenes().len();
        let count = scenes + 6;
        match key.code {
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Up | KeyCode::BackTab => self.row = (self.row + count - 1) % count,
            KeyCode::Down | KeyCode::Tab => self.row = (self.row + 1) % count,
            KeyCode::Home => self.row = 0,
            KeyCode::End => self.row = count - 1,
            KeyCode::Enter if self.row == count - 1 => return Action::Save(self.draft.clone()),
            KeyCode::Char(' ') | KeyCode::Enter if self.row < scenes => self.toggle_scene(),
            KeyCode::Char(' ') | KeyCode::Enter if self.row == scenes => {
                self.draft.enabled = !self.draft.enabled;
            }
            KeyCode::Char(' ') | KeyCode::Enter if self.row == scenes + 1 => {
                self.draft.fade = !self.draft.fade;
            }
            KeyCode::Left | KeyCode::Right => {
                let forward = key.code == KeyCode::Right;
                let rotate = |at: usize, len: usize| {
                    if forward {
                        (at + 1) % len
                    } else {
                        (at + len - 1) % len
                    }
                };
                if self.row < scenes {
                    self.toggle_scene();
                } else {
                    match self.row - scenes {
                        0 => self.draft.enabled = !self.draft.enabled,
                        1 => self.draft.fade = !self.draft.fade,
                        2 => {
                            let at = self.draft.level.map_or(0, |n| usize::from(n.min(10)) + 1);
                            self.draft.level = rotate(at, 12).checked_sub(1).map(|n| n as u8);
                        }
                        3 => {
                            let at = match self.draft.night {
                                None => 0,
                                Some(false) => 1,
                                Some(true) => 2,
                            };
                            self.draft.night = [None, Some(false), Some(true)][rotate(at, 3)];
                        }
                        4 => {
                            let at = COLORS
                                .iter()
                                .position(|(n, _)| *n == self.draft.color)
                                .unwrap_or(0);
                            self.draft.color = COLORS[rotate(at, COLORS.len())].0.to_string();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Action::Edit
    }

    pub fn rows(&self, width: usize, height: usize) -> Vec<String> {
        let scenes = scene::scenes();
        let mut labels: Vec<_> = scenes
            .iter()
            .map(|scene| {
                format!(
                    "[{}] {}",
                    if self.draft.disabled_scenes.iter().any(|n| n == scene.name) {
                        ' '
                    } else {
                        'x'
                    },
                    scene.name
                )
            })
            .collect();
        labels.extend([
            format!("[{}] Show", if self.draft.enabled { 'x' } else { ' ' }),
            format!("[{}] Fade", if self.draft.fade { 'x' } else { ' ' }),
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
        ]);
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
            } else if self.row < scenes.len() + 2 {
                "Space/↵ toggle"
            } else if self.row == labels.len() - 1 {
                "↵ save Esc back"
            } else {
                "←→ edit Esc"
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
    fn checklist_scrolls_and_can_save_every_scene_unchecked() {
        let original = FlowPreferences::default();
        let mut picker = Picker::new(original.clone());
        for scene in scene::scenes() {
            assert!(picker
                .rows(15, 4)
                .iter()
                .any(|r| r.contains(&format!("[x] {}", scene.name))));
            assert!(matches!(picker.key(key(KeyCode::Char(' '))), Action::Edit));
            assert!(picker
                .rows(15, 4)
                .iter()
                .any(|r| r.contains(&format!("[ ] {}", scene.name))));
            picker.key(key(KeyCode::Down));
        }
        assert!(
            original.disabled_scenes.is_empty(),
            "edits stay in the draft"
        );
        assert_eq!(picker.draft.disabled_scenes.len(), scene::scenes().len());
        picker.key(key(KeyCode::End));
        assert!(picker.rows(15, 4).iter().any(|r| r.contains("Save")));
        let Action::Save(saved) = picker.key(key(KeyCode::Enter)) else {
            panic!("save action");
        };
        assert_eq!(saved.disabled_scenes, picker.draft.disabled_scenes);
        let reopened = Picker::new(saved);
        assert!(reopened.rows(15, 4)[0].contains("[ ] fire"));
    }

    #[test]
    fn toggles_do_not_enable_show_or_save_and_escape_cancels() {
        let mut picker = Picker::new(FlowPreferences {
            enabled: false,
            ..FlowPreferences::default()
        });
        assert!(matches!(picker.key(key(KeyCode::Enter)), Action::Edit));
        assert_eq!(picker.draft.disabled_scenes, vec!["fire"]);
        assert!(!picker.draft.enabled);
        picker.key(key(KeyCode::Enter));
        assert!(picker.draft.disabled_scenes.is_empty());
        picker.row = scene::scenes().len();
        picker.key(key(KeyCode::Char(' ')));
        assert!(picker.draft.enabled);
        picker.key(key(KeyCode::Down));
        let fade = picker.draft.fade;
        picker.key(key(KeyCode::Char(' ')));
        assert_eq!(picker.draft.fade, !fade);
        assert!(matches!(picker.key(key(KeyCode::Esc)), Action::Cancel));
    }

    #[test]
    fn dials_keep_auto_and_manual_values_without_saving_on_enter() {
        let mut picker = Picker::new(FlowPreferences::default());
        picker.row = scene::scenes().len() + 2;
        picker.key(key(KeyCode::Left));
        assert_eq!(picker.draft.level, Some(10));
        picker.key(key(KeyCode::Right));
        assert_eq!(picker.draft.level, None);
        picker.key(key(KeyCode::Down));
        picker.key(key(KeyCode::Left));
        assert_eq!(picker.draft.night, Some(true));
        picker.key(key(KeyCode::Down));
        picker.key(key(KeyCode::Right));
        assert_eq!(picker.draft.color, "natural");
        assert!(matches!(picker.key(key(KeyCode::Enter)), Action::Edit));
        picker.key(key(KeyCode::Home));
        assert_eq!(picker.row, 0);
    }
}
