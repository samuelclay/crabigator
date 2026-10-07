// Records flow's own frames for the Rust port to match (src/flow/reference.rs).
//
// For each scene, a few runs from a fixed seed at a size, with the dials
// changing on the way (level, tint, night, subagents), and the frame's cells
// at chosen steps, base64 of [code point, fg, bg] per cell (little-endian
// u32s). Run through flow's own tsx:
//
//   make flow-reference FLOW_DIR=../flow            every scene
//   make flow-reference FLOW_DIR=../flow SCENES=fire  one scene

import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

type Dials = { level?: number; tint?: 'normal' | 'smoke' | 'blue'; night?: boolean; coverage?: number }
type Case = { columns: number; rows: number; seed: number; script: [number, Dials][]; snaps: number[] }

/** Runs every scene is checked on: the column's sizes, the band, the spine. */
const CASES: Case[] = [
  {
    columns: 24,
    rows: 8,
    seed: 7,
    script: [
      [0, { level: 3 }],
      [40, { level: 8 }],
      [90, { level: 5, tint: 'smoke' }],
      [130, { level: 1, tint: 'normal' }],
    ],
    snaps: [1, 20, 60, 120, 180],
  },
  {
    columns: 60,
    rows: 5,
    seed: 11,
    script: [
      [0, { level: 9, night: true }],
      [50, { level: 2, tint: 'blue' }],
      [100, { level: 7, coverage: 45, tint: 'normal' }],
    ],
    snaps: [5, 45, 100, 150],
  },
  {
    columns: 16,
    rows: 26,
    seed: 3,
    script: [
      [0, { level: 6, coverage: 30 }],
      [60, { level: 10, night: true }],
      [120, { level: 4, night: false, coverage: 0 }],
    ],
    snaps: [10, 70, 140, 200],
  },
  {
    columns: 36,
    rows: 10,
    seed: 1_759_861_234_567,
    script: [
      [0, { level: 10 }],
      [80, { level: 0 }],
      [100, { level: 7 }],
    ],
    snaps: [40, 90, 160],
  },
  // The full band and the full spine: room for what only shows there (the
  // engine's hammer and smokestack, its bell, a spine's summit and moon).
  {
    columns: 200,
    rows: 5,
    seed: 23,
    script: [
      [0, { level: 6 }],
      [70, { level: 10, coverage: 30 }],
      [130, { level: 3, tint: 'smoke', night: true }],
    ],
    snaps: [60, 160],
  },
  {
    columns: 22,
    rows: 60,
    seed: 5,
    script: [
      [0, { level: 8 }],
      [60, { level: 2, night: true, tint: 'blue' }],
    ],
    snaps: [50, 120],
  },
]

const flow = process.env.FLOW_DIR
if (!flow) throw new Error('FLOW_DIR: the flow checkout to read the scenes from')
const out = process.env.OUT ?? join(process.cwd(), 'src', 'flow', 'testdata')

async function main() {
  const { makeScene, STYLES } = await import(join(flow!, 'hooks', 'styles.ts'))
  const wanted = (process.env.SCENES ?? '').split(',').filter(Boolean)
  for (const scene of STYLES as string[]) {
    if (wanted.length && !wanted.includes(scene)) continue
    const cases = CASES.map(c => {
      const f = makeScene(scene, c.seed)
      const dials = { level: 8, tint: 'normal', night: false, coverage: 0 } as Required<Dials>
      const frames: string[] = []
      const last = Math.max(...c.snaps)
      for (let n = 1; n <= last; n++) {
        for (const [at, d] of c.script) if (at === n - 1) Object.assign(dials, d)
        f.strength = dials.level
        f.tint = dials.tint
        f.night = dials.night
        f.coverageBoost = dials.coverage
        f.ensure(c.columns, c.rows)
        f.step()
        if (f.sounds) f.sounds.length = 0
        if (c.snaps.includes(n)) frames.push(Buffer.from(f.grid().words.buffer).toString('base64'))
      }
      return { ...c, frames }
    })
    const file = join(out, `${scene}.json`)
    writeFileSync(file, `${JSON.stringify({ scene, cases })}\n`)
    console.log(`${file}: ${cases.length} runs`)
  }
}

main().catch(e => {
  console.error(e)
  process.exit(1)
})
