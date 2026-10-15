# Crabigator brand

A pixel crab and a pixel wordmark. Both use peach faces with a coral extrusion that falls down and to the left. Every file here is drawn by `scripts/render-brand.swift` from two pixel grids. To change the art, edit a grid in that script and run `swift scripts/render-brand.swift`. Don't edit these files by hand.

## Files

| File | What it is |
|---|---|
| `crabigator-logo.svg` | Crab and wordmark side by side. Use it wherever there's room. |
| `crabigator-wordmark.svg` | The name alone. |
| `crabigator-icon.svg` | The crab alone, with no background. |
| `crabigator-app-icon.svg` | The crab on ink, as on the phone home screens. |
| `favicon.svg`, `favicon.ico` | Browser tab icons. The .ico holds 16, 32 and 48 pixel sizes. |
| `png/` | Every image above as PNG, in several sizes. |

The PNG file names give the pixel size: `crabigator-logo-540x68.png` is 540 by 68 pixels.

## Keeping it sharp

Every pixel in the art has to land on a whole number of screen pixels. Show the images only at the sizes in `png/`, or scale an SVG by a whole number:

- Logo: 270×34 at 1×, 540×68 at 2×, and so on.
- Wordmark: 220×23 at 1×.
- Crab: 24×24 at 1×.

Turn off smoothing when scaling a PNG: `image-rendering: pixelated` on the web, or `.interpolation(.none)` in SwiftUI.

When the crab sits next to the wordmark, draw it at twice the wordmark's pixel size, as the logo does, so their extrusions match.

## Colors

| Color | Hex |
|---|---|
| Peach (faces) | `#FFB98A` |
| Coral (extrusion) | `#E8775A` |
| Cream (eye highlight) | `#FFE1B5` |
| Near black (eyes) | `#080E14` |
| Ink (app icon background) | `#141A21` |
