package com.crabigator.app

import org.junit.Assert.assertEquals
import org.junit.Test

class TerminalKeysTest {
    @Test fun encodesTerminalModifiersWithoutPastingTheirLabels() {
        assertEquals("\u001b[Z", terminalKey("Tab", shift = true, control = false, alt = false))
        assertEquals("\u0003", terminalKey("C", shift = false, control = true, alt = false))
        assertEquals("\u001b[1;3A", terminalKey("↑", shift = false, control = false, alt = true))
        assertEquals("\u001b[1;6D", terminalKey("←", shift = true, control = true, alt = false))
        assertEquals("\u001b[13;2u", terminalKey("Enter", shift = true, control = false, alt = false))
        assertEquals("\u001b[3~", terminalKey("Delete", shift = false, control = false, alt = false))
        assertEquals("\u001b\u007f", terminalKey("⌫", shift = false, control = true, alt = false))
        assertEquals("\u001b[9;5~", terminalKey("Tab", shift = false, control = true, alt = false))
        assertEquals("\r", terminalKey("Enter", shift = false, control = false, alt = false))
    }
}
