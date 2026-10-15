//! Procedural corridor textures and light, matching upstream's integer texels.
use crate::flow::js::{i32_of, round};
use crate::flow::pixels::{grey, hash, hash1, luma, mix, noise1, noise2, ramp_at};
pub(super) const SEG: f64 = 32.0;
pub(super) const UNIT: f64 = 16.0;
pub(super) const SPX: f64 = 0.095;
const COMPUTER: i32 = 1;
const WINDOW: i32 = 2;
const DOOR: i32 = 3;
const LAMP: i32 = 4;
const GREY: i32 = 5;
pub(super) const SKY: i32 = -2;
#[derive(Default)]
pub(super) struct Tex {
    pub lamp: bool,
    pub glow: bool,
}
pub(super) fn seg_kind(seg: f64, seed: f64) -> i32 {
    let h = hash1(seg * 7919.0 + seed);
    if h < 0.32 {
        0
    } else if h < 0.48 {
        COMPUTER
    } else if h < 0.6 {
        WINDOW
    } else if h < 0.7 {
        DOOR
    } else if h < 0.84 {
        LAMP
    } else {
        GREY
    }
}
pub(super) fn seg_light(seg: f64, t: f64, seed: f64) -> f64 {
    let mut l = 0.6 + 0.24 * hash1(seg * 977.0 + seed);
    if seg_kind(seg, seed) == LAMP {
        l += 0.2;
    }
    if hash1(seg * 131.0 + seed + 5.0) < 0.14
        && hash1((i32_of(f64::from(i32_of(t) >> 1) * 7.0 + seg * 13.0) ^ i32_of(seed)) as f64)
            < 0.22
    {
        l *= 0.5;
    }
    l
}
impl Tex {
    pub fn wall(&mut self, kind: i32, lx: f64, v: f64, seg: f64, t: f64, seed: f64) -> i32 {
        self.lamp = false;
        self.glow = false;
        let p = i32_of(lx) & 15;
        match kind {
            COMPUTER => {
                if lx < 2.0 || lx > 29.0 || v < 0.16 || v > 0.86 {
                    return if lx == 1.0 || lx == 30.0 {
                        0x3a3a38
                    } else {
                        0x5e5e5a
                    };
                }
                let row = (v * 11.0).floor();
                if i32_of(lx) & 3 == 2 && i32_of(row) & 1 == 0 {
                    let h = hash(seg * 64.0 + lx, row, seed);
                    if hash(lx + f64::from(i32_of(t) >> 3) * 17.0, row + seg * 5.0, seed) < 0.72 {
                        self.lamp = true;
                        return if h < 0.4 {
                            0x38e848
                        } else if h < 0.65 {
                            0xe83020
                        } else if h < 0.85 {
                            0xf0d040
                        } else {
                            0x40d8e8
                        };
                    }
                    return 0x2a3034;
                }
                return 0x1c2024;
            }
            WINDOW => {
                if lx < 4.0 || lx > 27.0 {
                    return if lx == 0.0 || lx == 31.0 {
                        0x3e2e1a
                    } else if i32_of(lx) & 1 != 0 && i32_of(v * 9.0) % 3 == 0 {
                        0x4a3820
                    } else {
                        0x70563a
                    };
                }
                if v < 0.2 || v > 0.86 {
                    return if v < 0.12 { 0x5c503e } else { 0x7e705a };
                }
                if lx == 4.0 || lx == 27.0 || v < 0.25 || v > 0.81 {
                    return 0x2a2622;
                }
                return SKY;
            }
            DOOR => {
                if v >= 0.82 {
                    return if p == 0 { 0x3e352a } else { 0x7f705a };
                }
                if lx == 4.0 || lx == 5.0 || lx == 26.0 || lx == 27.0 {
                    return if i32_of(lx + (v * 12.0).floor()) & 1 != 0 {
                        0xd8b020
                    } else {
                        0x201c14
                    };
                }
                if lx == 6.0 || lx == 25.0 {
                    return 0x4c4c48;
                }
                if lx == 15.0 || lx == 16.0 {
                    return 0x5a5852;
                }
                return if (v * 16.0).floor() % 3.0 == 0.0 {
                    0x6a6862
                } else {
                    0x8c8a82
                };
            }
            LAMP => {
                if lx >= 13.0 && lx <= 18.0 && v >= 0.5 && v <= 0.7 {
                    if lx >= 14.0 && lx <= 17.0 && v >= 0.54 && v <= 0.66 {
                        self.lamp = true;
                        return 0xfff4d0;
                    }
                    return 0x6a6a64;
                }
            }
            GREY => {
                if p == 0 {
                    return 0x3e3e3a;
                }
                if p == 1 {
                    return 0x9c9c96;
                }
                if v < 0.14 {
                    return 0x50504c;
                }
                return if hash(lx + seg * SEG, (v * 12.0).floor(), seed) < 0.1 {
                    0x6c6c68
                } else {
                    0x7c7c78
                };
            }
            _ => {}
        }
        if p == 0 {
            return 0x3e352a;
        }
        if p == 1 {
            return 0x9e8d70;
        }
        if v < 0.14 {
            return 0x5a4e3e;
        }
        if v > 0.6 && v < 0.72 {
            return 0x625642;
        }
        if hash(lx + seg * SEG, (v * 12.0).floor(), seed) < 0.1 {
            0x706450
        } else {
            0x7f705a
        }
    }
    pub fn sky(&mut self, sx: f64, v: f64, seed: f64) -> i32 {
        self.glow = true;
        let m = 0.3 + 0.32 * noise1(sx * 0.07, seed) + 0.12 * noise1(sx * 0.23, seed + 3.0);
        if v < m {
            if v < m - 0.12 {
                0x4a3e32
            } else {
                0x64544a
            }
        } else if v - m > 0.3 {
            0xa0a8b4
        } else {
            0x7c8898
        }
    }
    pub fn floor(&mut self, a: f64, lat: f64, t: f64, seed: f64) -> i32 {
        self.lamp = false;
        self.glow = false;
        let seg = (a / SEG).floor();
        let lx = a - seg * SEG;
        if hash1(seg * 31.0 + 7.0 + seed) < 0.16 && lx > 3.0 && lx < 28.0 && lat.abs() < 12.0 {
            if lx == 4.0 || lx == 27.0 || lat.abs() >= 11.0 {
                return 0x4a4a44;
            }
            self.glow = true;
            let n = noise2(a * 0.22 + t * 0.05, lat * 0.25 - t * 0.03, seed);
            return if n < 0.4 {
                0x1e6a10
            } else if n < 0.68 {
                0x36a01c
            } else {
                0x6ad83a
            };
        }
        let l = i32_of(lat.floor());
        let a = i32_of(a);
        if a & 15 == 0 || l & 15 == 0 {
            return 0x3a3428;
        }
        let tile = hash((a >> 4) as f64, (l >> 4) as f64, seed) < 0.5;
        let worn = hash((a >> 2) as f64, (l >> 2) as f64, seed + 1.0) < 0.22;
        if tile {
            if worn {
                0x52483a
            } else {
                0x5c5242
            }
        } else if worn {
            0x4c4436
        } else {
            0x564c3e
        }
    }
    pub fn ceil(&mut self, a: f64, lat: f64) -> i32 {
        self.lamp = false;
        self.glow = false;
        let m = ((a % 64.0) + 64.0) % 64.0;
        if m >= 28.0 && m < 36.0 && lat.abs() < 4.0 {
            self.glow = true;
            return 0xf0ecd8;
        }
        let l = i32_of(lat.floor());
        let a = i32_of(a);
        if a & 15 == 0 || l & 15 == 0 {
            return 0x2c2822;
        }
        if hash((a >> 2) as f64, (l >> 2) as f64, 77.0) < 0.2 {
            0x36302a
        } else {
            0x3c362e
        }
    }
}
pub(super) fn lit(c: u32, l: f64) -> u32 {
    if l == 1.0 {
        return c;
    }
    let r = i32_of((f64::from((c >> 16) & 255) * l).min(255.0));
    let g = i32_of((f64::from((c >> 8) & 255) * l).min(255.0));
    let b = i32_of((f64::from(c & 255) * l).min(255.0));
    ((r << 16) | (g << 8) | b) as u32
}
pub(super) fn q8(l: f64) -> f64 {
    round(l * 8.0) / 8.0
}
pub(super) fn tone(mut c: u32, s: f64, b: f64) -> u32 {
    if s > 0.0 {
        c = mix(c, lit(grey(c, 1.0), 0.42), s);
    }
    if b > 0.0 {
        c = mix(
            c,
            ramp_at(
                &[0x0c1a48, 0x2048a8, 0x5a8ae8, 0xb4d4ff, 0xf0f6ff],
                0.22 + 0.78 * (luma(c) / 255.0),
            ),
            b,
        );
    }
    c
}
