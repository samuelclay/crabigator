// The flow scenes (crates/flow-render), built for iOS by a build step in the app
// target. See crates/flow-render/src/lib.rs for what each call does.
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct Flow Flow;

/// A scene, or NULL for an unknown name. `hue`: NaN for the scene's own colours.
Flow *flow_new(const uint8_t *name, size_t len, uint32_t seed, double hue);
void flow_free(Flow *flow);
/// `tint`: 0 normal, 1 smoke, 2 blue.
void flow_set(Flow *flow, double level, bool waiting, bool night, uint32_t tint, double boost);
/// Step once; `columns * rows * 3` words (code point, foreground, background),
/// valid until the next call. A colour of 0x01000000 is the background behind.
const uint32_t *flow_frame(Flow *flow, uint32_t columns, uint32_t rows);
/// Milliseconds until the next frame.
uint32_t flow_pace(Flow *flow);
