//! Individual subagent states supplied to scene companions.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    Working,
    Idle,
    Waiting,
    Done,
}
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct AgentDial {
    pub id: String,
    pub state: AgentState,
    pub ok: bool,
    #[serde(default)]
    pub task: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub ms: f64,
}
