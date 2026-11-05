//! Crabigator's flow scenes, built from the CLI's own scene code (see `flow.rs`)
//! for the iOS and Android apps and the web dashboard. A client makes a scene
//! with the session's scene name, seed, and hue (as the desktop publishes them),
//! sets its dials as the session works, and draws each frame's cells: per cell
//! a code point, a foreground, and a background (`0x01000000`: the terminal's
//! own background), drawn with flow's 2x4 masks.
#![allow(dead_code)]

pub mod flow;

use flow::cells::Cells;
use flow::palette::Palette;
use flow::scene::{scene_def, Scene, SceneDef, Tint};

/// How long a frame lasts: as the CLI paces the column (src/flow.rs).
const BUSY_MS: u32 = 70;
const CALM_MS: u32 = 125;

pub struct Flow {
    def: &'static SceneDef,
    scene: Box<dyn Scene>,
    palette: Palette,
    turned: Cells,
    out: Vec<u32>,
}

impl Flow {
    /// The scene `name` from `seed`, in the session's `hue` (OKLCH degrees;
    /// `None`: its own colours). `None` for a scene this build doesn't know.
    pub fn new(name: &str, seed: u32, hue: Option<f64>) -> Option<Self> {
        let def = scene_def(name)?;
        let mut scene = (def.make)(f64::from(seed));
        scene.dials().accent = if def.figure { hue } else { None };
        // A figure scene's figure wears the hue; any other scene turns toward it.
        let palette = if def.figure { Palette::natural() } else { Palette::toward(def.hue, hue) };
        Some(Self { def, scene, palette, turned: Cells::new(0, 0), out: Vec::new() })
    }

    /// The session's activity, as the desktop's column has it.
    pub fn set(&mut self, level: f64, waiting: bool, night: bool, tint: Tint, boost: f64) {
        let dials = self.scene.dials();
        dials.strength = level;
        dials.waiting = waiting;
        dials.night = night;
        dials.tint = tint;
        dials.coverage_boost = boost;
    }

    /// Step once and return the frame: `columns * rows * 3` words.
    pub fn frame(&mut self, columns: usize, rows: usize) -> &[u32] {
        let (tint, waiting) = {
            let dials = self.scene.dials();
            (dials.tint, dials.waiting)
        };
        self.scene.ensure(columns, rows);
        self.scene.step();
        let grid = self.scene.grid();
        // The context-full tint must read as blue: no palette then, as in the CLI.
        let words = if self.palette.is_natural() || tint == Tint::Blue || waiting {
            &grid.words
        } else {
            self.palette.apply(grid, &mut self.turned);
            &self.turned.words
        };
        self.out.clear();
        self.out.extend_from_slice(words);
        &self.out
    }

    /// Milliseconds until the next frame: calm scenes step slower.
    pub fn pace(&mut self) -> u32 {
        let dials = self.scene.dials();
        if dials.strength <= 1.0 && dials.tint == Tint::Normal {
            CALM_MS
        } else {
            BUSY_MS
        }
    }
}

fn tint_of(code: u32) -> Tint {
    match code {
        1 => Tint::Smoke,
        2 => Tint::Blue,
        _ => Tint::Normal,
    }
}

fn hue_of(hue: f64) -> Option<f64> {
    hue.is_finite().then_some(hue)
}

// C interface, for the iOS app (a static library) and the dashboard (WebAssembly).

/// A scene, or null for an unknown name. `hue`: NaN for the scene's own colours.
///
/// # Safety
/// `name` must point to `len` readable bytes of UTF-8.
#[no_mangle]
pub unsafe extern "C" fn flow_new(name: *const u8, len: usize, seed: u32, hue: f64) -> *mut Flow {
    let bytes = std::slice::from_raw_parts(name, len);
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|name| Flow::new(name, seed, hue_of(hue)))
        .map_or(std::ptr::null_mut(), |flow| Box::into_raw(Box::new(flow)))
}

/// # Safety
/// `flow` must come from `flow_new` and not be used again.
#[no_mangle]
pub unsafe extern "C" fn flow_free(flow: *mut Flow) {
    if !flow.is_null() {
        drop(Box::from_raw(flow));
    }
}

/// `tint`: 0 normal, 1 smoke, 2 blue.
///
/// # Safety
/// `flow` must come from `flow_new`.
#[no_mangle]
pub unsafe extern "C" fn flow_set(flow: *mut Flow, level: f64, waiting: bool, night: bool, tint: u32, boost: f64) {
    if let Some(flow) = flow.as_mut() {
        flow.set(level, waiting, night, tint_of(tint), boost);
    }
}

/// Step once; the frame's `columns * rows * 3` words, valid until the next call.
///
/// # Safety
/// `flow` must come from `flow_new`.
#[no_mangle]
pub unsafe extern "C" fn flow_frame(flow: *mut Flow, columns: u32, rows: u32) -> *const u32 {
    match flow.as_mut() {
        Some(flow) => flow.frame(columns as usize, rows as usize).as_ptr(),
        None => std::ptr::null(),
    }
}

/// # Safety
/// `flow` must come from `flow_new`.
#[no_mangle]
pub unsafe extern "C" fn flow_pace(flow: *mut Flow) -> u32 {
    flow.as_mut().map_or(CALM_MS, Flow::pace)
}

/// Room for a scene name, for callers that can't pass a pointer of their own
/// (WebAssembly): write the name here, then call `flow_new` with it.
#[no_mangle]
pub extern "C" fn flow_name_buffer() -> *mut u8 {
    static mut NAME: [u8; 32] = [0; 32];
    std::ptr::addr_of_mut!(NAME).cast()
}

#[cfg(feature = "android")]
mod android {
    //! The JNI entry points behind `com.crabigator.app.FlowNative`.
    use super::{hue_of, tint_of, Flow};
    use jni::objects::{JClass, JIntArray, JString};
    use jni::sys::{jboolean, jdouble, jint, jlong};
    use jni::JNIEnv;

    #[no_mangle]
    pub extern "system" fn Java_com_crabigator_app_FlowNative_create(
        mut env: JNIEnv,
        _: JClass,
        name: JString,
        seed: jint,
        hue: jdouble,
    ) -> jlong {
        let Ok(name) = env.get_string(&name) else { return 0 };
        let name: String = name.into();
        Flow::new(&name, seed as u32, hue_of(hue)).map_or(0, |flow| Box::into_raw(Box::new(flow)) as jlong)
    }

    #[no_mangle]
    pub extern "system" fn Java_com_crabigator_app_FlowNative_free(_: JNIEnv, _: JClass, handle: jlong) {
        if handle != 0 {
            drop(unsafe { Box::from_raw(handle as *mut Flow) });
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_com_crabigator_app_FlowNative_set(
        _: JNIEnv,
        _: JClass,
        handle: jlong,
        level: jdouble,
        waiting: jboolean,
        night: jboolean,
        tint: jint,
        boost: jdouble,
    ) {
        if let Some(flow) = unsafe { (handle as *mut Flow).as_mut() } {
            flow.set(level, waiting != 0, night != 0, tint_of(tint as u32), boost);
        }
    }

    /// Step once into `out` (`columns * rows * 3` ints); returns the pace in ms.
    #[no_mangle]
    pub extern "system" fn Java_com_crabigator_app_FlowNative_frame(
        env: JNIEnv,
        _: JClass,
        handle: jlong,
        columns: jint,
        rows: jint,
        out: JIntArray,
    ) -> jint {
        let Some(flow) = (unsafe { (handle as *mut Flow).as_mut() }) else { return 125 };
        let words: Vec<i32> = flow.frame(columns as usize, rows as usize).iter().map(|&w| w as i32).collect();
        let _ = env.set_int_array_region(&out, 0, &words);
        flow.pace() as jint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scene_draws_a_full_frame() {
        for def in flow::scene::scenes() {
            let mut flow = Flow::new(def.name, 737, Some(305.0)).expect(def.name);
            flow.set(6.0, false, false, Tint::Normal, 0.0);
            // Some scenes (fire) start empty and build up, so run a few seconds first.
            for _ in 0..120 {
                flow.frame(18, 6);
            }
            let words = flow.frame(18, 6).to_vec();
            assert_eq!(words.len(), 18 * 6 * 3, "{}", def.name);
            assert!(words.chunks_exact(3).any(|cell| cell[0] != 0x20 || cell[2] != 0x0100_0000), "{} drew nothing", def.name);
        }
    }

    #[test]
    fn calm_scenes_step_slower_and_unknown_names_make_nothing() {
        let mut flow = Flow::new("fire", 1, None).unwrap();
        flow.set(1.0, false, false, Tint::Normal, 0.0);
        assert_eq!(flow.pace(), CALM_MS);
        flow.set(7.0, false, false, Tint::Normal, 0.0);
        assert_eq!(flow.pace(), BUSY_MS);
        assert!(Flow::new("no such scene", 1, None).is_none());
    }
}
