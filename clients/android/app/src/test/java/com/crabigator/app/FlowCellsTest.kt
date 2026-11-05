package com.crabigator.app

import org.junit.Assert.*
import org.junit.Test

class FlowCellsTest {
    @Test fun blockGlyphsLightTheirSubBlocks() {
        assertEquals(0xFF, FlowCells.mask(0x2588))      // full block
        assertEquals(0x0F, FlowCells.mask(0x2580))      // upper half: the top two of four rows
        assertEquals(0xF0, FlowCells.mask(0x2584))      // lower half
        assertEquals(0x55, FlowCells.mask(0x258C))      // left half: the left column
        assertEquals(0xFF, FlowCells.mask(0x28FF))      // every braille dot
        assertEquals(0, FlowCells.mask(0x20))
        assertNull(FlowCells.mask('A'.code))            // drawn as a glyph instead
    }

    @Test fun clearColoursLetTheBackdropShow() {
        val red = 0xFF0000
        val sub = FlowCells.subBlocks(intArrayOf(0x2580, red, FlowCells.CLEAR), 1, 1)
        assertEquals(listOf(red, red, red, red, -1, -1, -1, -1), sub.toList())
    }
}
