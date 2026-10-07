//! The cell grid every scene draws into, and the small PRNG they flicker by
//! (flow's `hooks/cells.ts`).

use super::js::u32_of;

/// xorshift32: only the flicker depends on it.
#[derive(Clone, Debug)]
pub struct Rng {
    s: u32,
}

impl Rng {
    /// `new Rng(seed)`: `(seed | 1) >>> 0 || 0x9e3779b9`.
    pub fn new(seed: f64) -> Self {
        let s = u32_of(seed) | 1;
        Self {
            s: if s == 0 { 0x9e37_79b9 } else { s },
        }
    }

    pub fn int(&mut self) -> u32 {
        let mut x = self.s;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.s = x;
        x
    }

    /// A float in [0, 1).
    pub fn f(&mut self) -> f64 {
        f64::from(self.int()) / 4_294_967_296.0
    }
}

/// Whether a region gets a scene's tall layout (the spine's, a pane's)
/// rather than its band layout. A small box up to 12 rows (this column)
/// goes by how it looks: a cell is about twice as tall as it is wide.
pub fn is_tall(columns: usize, rows: usize) -> bool {
    rows > 12 || rows > columns || (rows > 8 && rows * 2 > columns)
}

/// The terminal's own color: transparent.
pub const DEFAULT_COLOR: u32 = 0x0100_0000;
const SPACE: u32 = 0x20;

/// A columns × rows grid of cells: [code point, fg, bg] each, colors 0xRRGGBB.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cells {
    pub columns: usize,
    pub rows: usize,
    pub words: Vec<u32>,
}

impl Cells {
    pub fn new(columns: usize, rows: usize) -> Self {
        Self {
            columns,
            rows,
            words: vec![0; columns * rows * 3],
        }
    }

    pub fn len(&self) -> usize {
        self.columns * self.rows
    }

    /// Out of range does nothing, as a write past a JS typed array's end.
    pub fn set(&mut self, i: usize, code_point: u32, fg: u32, bg: u32) {
        if let Some(cell) = self.words.get_mut(i * 3..i * 3 + 3) {
            cell.copy_from_slice(&[code_point, fg, bg]);
        }
    }

    /// `set` with the terminal's own background.
    pub fn set_fg(&mut self, i: usize, code_point: u32, fg: u32) {
        self.set(i, code_point, fg, DEFAULT_COLOR);
    }

    pub fn blank(&mut self, i: usize) {
        self.set(i, SPACE, DEFAULT_COLOR, DEFAULT_COLOR);
    }

    /// Out of range reads 0.
    pub fn code_point(&self, i: usize) -> u32 {
        self.words.get(i * 3).copied().unwrap_or(0)
    }

    pub fn foreground(&self, i: usize) -> u32 {
        self.words.get(i * 3 + 1).copied().unwrap_or(0)
    }

    pub fn background(&self, i: usize) -> u32 {
        self.words.get(i * 3 + 2).copied().unwrap_or(0)
    }

    /// The one color a cell mostly shows: a full block its foreground, else its background.
    pub fn behind(&self, i: usize) -> u32 {
        if self.code_point(i) == 0x2588 {
            self.foreground(i)
        } else {
            self.background(i)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rng_matches_flows() {
        // new Rng(7): 1st..3rd int() in flow's TypeScript.
        let mut rng = Rng::new(7.0);
        assert_eq!(
            [rng.int(), rng.int(), rng.int()],
            [1_892_583, 470_389_255, 3_882_205_507]
        );
        assert_eq!(Rng::new(0.0).int(), Rng::new(1.0).int());
    }
}
