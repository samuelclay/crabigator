//! Stable companion slots and eased arrival, rest and departure, ported from Flow.
use super::agents::{AgentDial, AgentState};
use super::pixels::smooth;
#[derive(Clone, Debug)]
pub struct Mate {
    pub id: String,
    pub slot: usize,
    pub here: f64,
    pub busy: f64,
    pub waiting: bool,
    pub leaving: bool,
    pub ok: bool,
    pub age: f64,
    pub seed: f64,
    pub x: f64,
    pub y: f64,
    pub p: f64,
}
pub fn seed_of(id: &str) -> f64 {
    let mut h = 0x811c9dc5u32;
    for c in id.encode_utf16() {
        h = (h ^ u32::from(c)).wrapping_mul(0x01000193);
    }
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    f64::from(h) / 4294967296.0
}
pub fn resting(m: Option<&Mate>) -> bool {
    m.is_some_and(|m| !m.leaving && m.busy < 0.5)
}
pub fn finished(m: Option<&Mate>) -> bool {
    m.is_some_and(|m| m.leaving && m.ok)
}
pub fn failed(m: Option<&Mate>) -> bool {
    m.is_some_and(|m| m.leaving && !m.ok)
}
pub fn beckon(m: &Mate, t: f64) -> bool {
    ((super::js::i32_of(t + m.slot as f64 * 3.0) >> 2) % 2) == 0
}
pub fn ease_to(v: f64, goal: f64, k: f64) -> f64 {
    if v.is_nan() {
        goal
    } else {
        v + (goal - v) * k
    }
}
#[derive(Clone, Debug)]
pub struct Crew {
    pub capacity: usize,
    pub arrive: f64,
    pub leave: f64,
    pub room: usize,
    pub mates: Vec<Mate>,
}
impl Crew {
    pub fn new(capacity: usize, arrive: f64, leave: f64) -> Self {
        Self {
            capacity,
            arrive,
            leave,
            room: capacity,
            mates: Vec::new(),
        }
    }
    pub fn in_slot(&self, slot: usize) -> Option<&Mate> {
        self.mates.iter().find(|m| m.slot == slot)
    }
    pub fn update(&mut self, agents: &[AgentDial], coverage: f64) {
        let anonymous;
        let list = if agents.is_empty() {
            anonymous = (0..super::js::round(coverage / 15.0).max(0.0) as usize)
                .map(|i| AgentDial {
                    id: format!("~{i}"),
                    state: AgentState::Working,
                    ok: true,
                    task: String::new(),
                    kind: String::new(),
                    ms: 0.0,
                })
                .collect::<Vec<_>>();
            &anonymous[..]
        } else {
            agents
        };
        for m in &mut self.mates {
            let a = list.iter().find(|a| a.id == m.id);
            let gone = a.is_none_or(|a| a.state == AgentState::Done);
            if gone && !m.leaving {
                m.ok = a.is_none_or(|a| a.ok);
            }
            m.leaving = gone;
            m.waiting = !gone && a.is_some_and(|a| a.state == AgentState::Waiting);
            let goal = if !gone && a.is_some_and(|a| a.state == AgentState::Working) {
                1.0
            } else {
                0.0
            };
            m.busy += (goal - m.busy) * 0.08;
            if (goal - m.busy).abs() < 0.01 {
                m.busy = goal;
            }
            m.p = (m.p
                + if m.leaving {
                    -1.0 / self.leave
                } else {
                    1.0 / self.arrive
                })
            .clamp(0.0, 1.0);
            m.here = smooth(m.p);
            m.age += 1.0;
        }
        let room = self.room.min(self.capacity);
        self.mates
            .retain(|m| !(m.leaving && m.p <= 0.0) && m.slot < room);
        for a in list {
            if a.state == AgentState::Done || self.mates.iter().any(|m| m.id == a.id) {
                continue;
            }
            let Some(slot) = (0..room).find(|&s| self.in_slot(s).is_none()) else {
                break;
            };
            self.mates.push(Mate {
                id: a.id.clone(),
                slot,
                here: 0.0,
                busy: if a.state == AgentState::Working {
                    1.0
                } else {
                    0.0
                },
                waiting: a.state == AgentState::Waiting,
                leaving: false,
                ok: true,
                age: 0.0,
                seed: seed_of(&a.id),
                x: f64::NAN,
                y: f64::NAN,
                p: 0.0,
            });
        }
    }
}
