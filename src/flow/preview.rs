//! Manual scene review: emit the Rust renderer's actual cells and timings.
//!
//! FLOW_PREVIEW_OUT=/tmp/flow-preview cargo test --release
//! flow::preview::render_all_scenes -- --ignored --nocapture
//!
//! Optional FLOW_PREVIEW_SCENES=train,earthrise restricts the review. Open
//! index.html to inspect every layout, tint and companion state. frames.json
//! carries cells for a pixel renderer or reference comparison. Set
//! FLOW_PREVIEW_HUE=305 to review the finished session palette, including the
//! same waiting/blue bypass as FlowColumn. Omit it for untouched scene cells.
use super::agents::{AgentDial, AgentState};
use super::scene::{scenes, Dials, Tint};
use serde_json::json;
use std::collections::HashSet;
use std::fmt::Write;
use std::time::Instant;

fn agent(id: &str, state: AgentState, ok: bool) -> AgentDial {
    AgentDial {
        id: id.into(),
        state,
        ok,
        task: format!("Preview {id}"),
        kind: "worker".into(),
        ms: 30000.0,
    }
}

fn css_color(color: u32) -> String {
    if color == super::cells::DEFAULT_COLOR {
        "transparent".into()
    } else {
        format!("#{color:06x}")
    }
}

#[test]
#[ignore = "manual visual and performance review; writes HTML and frame JSON"]
fn render_all_scenes() {
    let out = std::path::PathBuf::from(
        std::env::var("FLOW_PREVIEW_OUT").unwrap_or_else(|_| "/tmp/flow-preview".into()),
    );
    std::fs::create_dir_all(&out).unwrap();
    let selected = std::env::var("FLOW_PREVIEW_SCENES").unwrap_or_default();
    let hue = std::env::var("FLOW_PREVIEW_HUE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok());
    let modes = [
        ("idle", 1.0, false, Tint::Normal, false),
        ("working", 5.0, false, Tint::Normal, false),
        ("busy", 10.0, false, Tint::Normal, false),
        ("night", 5.0, true, Tint::Normal, false),
        ("smoke-idle", 1.0, false, Tint::Smoke, false),
        ("smoke-busy", 10.0, false, Tint::Smoke, false),
        ("blue-idle", 1.0, false, Tint::Blue, false),
        ("blue-busy", 10.0, false, Tint::Blue, false),
        ("waiting", 2.0, false, Tint::Normal, true),
        ("companions", 6.0, false, Tint::Normal, false),
    ];
    let mut frames = Vec::new();
    let mut html = String::from("<!doctype html><html><meta charset='utf-8'><title>Rust Flow scene review</title><style>body{background:#10151f;color:#d9e2f1;font:14px system-ui;margin:24px}h1{font-size:24px}h2{margin-top:36px}h3{font-size:14px}section{display:flex;flex-wrap:wrap;gap:16px;align-items:start}article{border:1px solid #364158;border-radius:6px;padding:12px;background:#181f2b}pre{font-family:'Menlo','DejaVu Sans Mono',monospace;font-size:14px;line-height:16.86px;letter-spacing:0;margin:12px 0 0}pre span{display:inline-block;width:8.43px}small{color:#a0aec5}</style><h1>Rust Flow scenes</h1><p>Actual Rust cell grids. Dimensions stay within Crabigator's existing column. Each timing includes step + grid after warmup.</p>");
    for def in scenes() {
        if !selected.is_empty() && !selected.split(',').any(|name| name == def.name) {
            continue;
        }
        writeln!(html, "<h2>{}</h2>", def.name).unwrap();
        for (mode, level, night, tint, waiting) in modes {
            writeln!(html, "<h3>{mode}</h3><section>").unwrap();
            for (columns, rows) in [(15, 4), (24, 8), (40, 12)] {
                let mut scene = (def.make)(737.0);
                *scene.dials() = Dials {
                    strength: if waiting { 8.0 } else { level },
                    night,
                    tint,
                    accent: if def.figure { hue } else { None },
                    ..Dials::default()
                };
                let start = Instant::now();
                scene.ensure(columns, rows);
                let init_ms = start.elapsed().as_secs_f64() * 1000.0;
                for _ in 0..240 {
                    scene.step();
                    scene.grid();
                }
                if waiting {
                    scene.dials().strength = level;
                    scene.dials().waiting = true;
                }
                if mode == "companions" {
                    scene.dials().agents = (0..4)
                        .map(|i| agent(&format!("agent-{i}"), AgentState::Working, true))
                        .collect();
                    scene.dials().coverage_boost = 60.0;
                }
                for _ in 0..300 {
                    scene.step();
                    scene.grid();
                }
                if mode == "companions" {
                    let a = &mut scene.dials().agents;
                    a[1].state = AgentState::Idle;
                    a[2].state = AgentState::Waiting;
                    for _ in 0..70 {
                        scene.step();
                        scene.grid();
                    }
                }
                let mut samples = Vec::new();
                for _ in 0..32 {
                    let t = Instant::now();
                    scene.step();
                    scene.grid();
                    samples.push(t.elapsed().as_secs_f64() * 1000.0);
                }
                samples.sort_by(f64::total_cmp);
                let mean_ms = samples.iter().sum::<f64>() / samples.len() as f64;
                let p95_ms = samples[30];
                let mut palette = super::palette_for(def, hue);
                let mut turned = super::cells::Cells::new(columns, rows);
                let grid = scene.grid();
                let grid = if palette.is_natural() || tint == Tint::Blue || waiting {
                    grid
                } else {
                    palette.apply(grid, &mut turned);
                    &turned
                };
                let pairs = grid
                    .words
                    .chunks_exact(3)
                    .map(|c| (c[1], c[2]))
                    .collect::<HashSet<_>>()
                    .len();
                println!("{:9} {:12} {columns:2}×{rows:2} init {init_ms:7.2}ms mean {mean_ms:6.3}ms p95 {p95_ms:6.3}ms {pairs:4} colors",def.name,mode);
                writeln!(html,"<article><b>{columns} × {rows}</b> <small>mean {mean_ms:.3} ms · p95 {p95_ms:.3} ms · {pairs} color pairs</small><pre>").unwrap();
                for row in grid.words.chunks_exact(columns * 3) {
                    for cell in row.chunks_exact(3) {
                        let glyph = char::from_u32(cell[0]).unwrap_or(' ');
                        let glyph = match glyph {
                            '&' => "&amp;".into(),
                            '<' => "&lt;".into(),
                            '>' => "&gt;".into(),
                            _ => glyph.to_string(),
                        };
                        write!(
                            html,
                            "<span style='color:{};background:{}'>{glyph}</span>",
                            css_color(cell[1]),
                            css_color(cell[2])
                        )
                        .unwrap();
                    }
                    html.push('\n');
                }
                html.push_str("</pre></article>");
                frames.push(json!({"scene":def.name,"hue":hue,"mode":mode,"columns":columns,"rows":rows,"init_ms":init_ms,"mean_ms":mean_ms,"p95_ms":p95_ms,"pairs":pairs,"words":grid.words}));
            }
            html.push_str("</section>");
        }
    }
    html.push_str("</html>");
    std::fs::write(out.join("index.html"), html).unwrap();
    std::fs::write(
        out.join("frames.json"),
        serde_json::to_vec(&frames).unwrap(),
    )
    .unwrap();
    println!("Preview: {}", out.join("index.html").display());
}
