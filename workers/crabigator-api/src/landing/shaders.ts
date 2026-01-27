// WebGL shader source code for terminal texture patterns

export const vertexShader = `
attribute vec2 a_position;
void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

// Shared utility functions for all fragment shaders
const shaderUtils = `
precision mediump float;
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_opacity;
uniform vec3 u_color;

float random(vec2 st) {
    return fract(sin(dot(st, vec2(12.9898, 78.233))) * 43758.5453);
}

float noise(vec2 st) {
    vec2 i = floor(st);
    vec2 f = fract(st);
    float a = random(i);
    float b = random(i + vec2(1.0, 0.0));
    float c = random(i + vec2(0.0, 1.0));
    float d = random(i + vec2(1.0, 1.0));
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}
`;

// 1. Dot Grid - LED matrix style dots
export const dotGridShader = `${shaderUtils}
void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution;
    vec2 grid = fract(uv * u_resolution / 16.0);
    float dist = length(grid - 0.5);
    float dot = smoothstep(0.12, 0.08, dist);

    // Add subtle brightness variation
    vec2 cell = floor(uv * u_resolution / 16.0);
    float variation = 0.7 + 0.3 * random(cell);

    gl_FragColor = vec4(u_color, dot * u_opacity * variation);
}
`;

// 2. Scanlines - CRT monitor horizontal lines
export const scanlinesShader = `${shaderUtils}
void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution;
    float y = gl_FragCoord.y;

    // Primary scanlines
    float line = 1.0 - step(0.5, fract(y / 3.0));

    // Subtle brightness wave
    float wave = 0.95 + 0.05 * sin(y * 0.02 + u_time * 0.5);

    // Very subtle horizontal glow variation
    float glow = 0.8 + 0.2 * sin(uv.x * 6.28318 + u_time * 0.3);

    gl_FragColor = vec4(u_color, line * u_opacity * wave * glow);
}
`;

// 3. Cross-hatch - Technical blueprint diagonal lines
export const crosshatchShader = `${shaderUtils}
void main() {
    vec2 pos = gl_FragCoord.xy;

    float spacing = 24.0;
    float lineWidth = 1.0;

    // Diagonal lines at +45 and -45 degrees
    float d1 = abs(mod(pos.x + pos.y, spacing) - spacing * 0.5);
    float d2 = abs(mod(pos.x - pos.y, spacing) - spacing * 0.5);

    float line1 = 1.0 - smoothstep(0.0, lineWidth, d1);
    float line2 = 1.0 - smoothstep(0.0, lineWidth, d2);

    // Combine with slightly brighter intersections
    float pattern = line1 + line2 - line1 * line2 * 0.5;

    gl_FragColor = vec4(u_color, pattern * u_opacity);
}
`;

// 4. ASCII Noise - Scattered terminal character blocks
export const asciiNoiseShader = `${shaderUtils}
float charGlyph(vec2 uv, float seed) {
    // Create blocky character-like shapes
    vec2 grid = floor(uv * 4.0);
    float r = random(grid + seed);
    return step(0.4, r);
}

void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution;
    float charSize = 14.0;

    vec2 cell = floor(gl_FragCoord.xy / charSize);
    float cellRand = random(cell);

    // Only show characters in ~5% of cells
    if (cellRand > 0.05) {
        discard;
    }

    // Local UV within character cell
    vec2 localUV = fract(gl_FragCoord.xy / charSize);

    // Generate character shape
    float char = charGlyph(localUV, cellRand * 100.0);

    // Add some variation in brightness
    float brightness = 0.6 + 0.4 * random(cell + 0.5);

    gl_FragColor = vec4(u_color, char * u_opacity * brightness);
}
`;

// 5. Vertical Lines - RGB subpixel columns
export const verticalLinesShader = `${shaderUtils}
void main() {
    float x = gl_FragCoord.x;
    float spacing = 3.0;

    float col = mod(floor(x / spacing), 3.0);
    float line = 1.0 - step(0.7, fract(x / spacing));

    // RGB subpixel colors
    vec3 color;
    if (col < 0.5) {
        color = vec3(1.0, 0.3, 0.3); // Red
    } else if (col < 1.5) {
        color = vec3(0.3, 1.0, 0.3); // Green
    } else {
        color = vec3(0.3, 0.5, 1.0); // Blue
    }

    gl_FragColor = vec4(color, line * u_opacity * 0.7);
}
`;

// 6. Binary Rain - Matrix-style falling characters
export const binaryRainShader = `${shaderUtils}
void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution;
    float colWidth = 14.0;
    float charHeight = 16.0;

    float col = floor(gl_FragCoord.x / colWidth);
    float colRand = random(vec2(col, 0.0));

    // Only ~15% of columns are active
    if (colRand > 0.15) {
        discard;
    }

    // Falling animation - each column falls at different speed
    float speed = 0.03 + colRand * 0.05;
    float fall = fract(uv.y + u_time * speed);

    // Fade at top and bottom of the trail
    float fade = smoothstep(0.0, 0.2, fall) * smoothstep(1.0, 0.6, fall);

    // Character grid within column
    float charY = floor(fall * u_resolution.y / charHeight);
    float charRand = random(vec2(col, charY + floor(u_time * 2.0)));

    // Simple blocky character shape
    vec2 localUV = fract(gl_FragCoord.xy / vec2(colWidth, charHeight));
    float inChar = step(0.15, localUV.x) * step(localUV.x, 0.85) *
                   step(0.1, localUV.y) * step(localUV.y, 0.9);

    // Random character pattern
    float char = step(0.4, charRand) * inChar;

    gl_FragColor = vec4(u_color, char * fade * u_opacity);
}
`;

// 7. Neon Palm Tree - Matches SVG reference with drooping fronds
export const neonPalmShader = `${shaderUtils}

vec2 bezier(vec2 a, vec2 b, vec2 c, float t) {
    return (1.0-t)*(1.0-t)*a + 2.0*(1.0-t)*t*b + t*t*c;
}

// Distance to bezier curve (moderate samples for slight pixelation)
float sdBezier(vec2 p, vec2 a, vec2 b, vec2 c) {
    float minDist = 1000.0;
    for (float i = 0.0; i <= 32.0; i += 1.0) {
        float t = i / 32.0;
        vec2 pt = bezier(a, b, c, t);
        minDist = min(minDist, length(p - pt));
    }
    return minDist;
}

// Leaf using bezier spine with varying width
float palmLeaf(vec2 p, vec2 start, vec2 ctrl, vec2 end, float maxWidth) {
    float minDist = 1000.0;
    float bestT = 0.0;

    for (float i = 0.0; i <= 32.0; i += 1.0) {
        float t = i / 32.0;
        vec2 pt = bezier(start, ctrl, end, t);
        float d = length(p - pt);
        if (d < minDist) {
            minDist = d;
            bestT = t;
        }
    }

    // Leaf width: sin^0.6 keeps sharp tips but spreads middle
    float width = maxWidth * pow(sin(bestT * 3.14159), 0.6);
    return minDist - width;
}

void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution;
    float aspect = u_resolution.x / u_resolution.y;
    vec2 p = vec2(uv.x * aspect, uv.y);

    // Palm position - right side
    float palmX = aspect * 0.75;

    // Sway animation - affects whole tree
    float sway = sin(u_time * 0.5) * 0.03;
    float sway2 = sin(u_time * 0.4 + 0.5) * 0.015;

    // === CURVED TRUNK ===
    // Wind-swept palm: concave up (positive second derivative)
    // Control point below the line from base to crown creates the bowl/smile curve
    vec2 trunkBot = vec2(palmX - 0.36, -0.1);
    vec2 trunkMid = vec2(palmX + 0.02 + sway * 0.4, 0.20); // Right and low for concave-up
    vec2 trunkTop = vec2(palmX + 0.01 + sway, 0.68); // Meets crown

    // Distance to curved trunk spine
    float trunkDist = sdBezier(p, trunkBot, trunkMid, trunkTop);

    // Find t along trunk for tapering (moderate samples)
    float bestT = 0.0;
    float minD = 1000.0;
    for (float i = 0.0; i <= 32.0; i += 1.0) {
        float t = i / 32.0;
        vec2 pt = bezier(trunkBot, trunkMid, trunkTop, t);
        float d = length(p - pt);
        if (d < minD) { minD = d; bestT = t; }
    }

    // Wider trunk that tapers
    float trunkW = mix(0.028, 0.012, bestT);
    float inTrunk = smoothstep(trunkW + 0.003, trunkW - 0.003, trunkDist);

    // Bark texture - rough scaly palm bark
    // Multiple noise scales for organic roughness
    float coarseNoise = noise(vec2(p.x * 80.0, p.y * 25.0));
    float fineNoise = noise(vec2(p.x * 150.0, p.y * 60.0));
    float detailNoise = noise(vec2(p.x * 300.0, p.y * 120.0));

    // Combine for rough bark look
    float bark = coarseNoise * 0.5 + fineNoise * 0.35 + detailNoise * 0.15;

    // Add some horizontal segment hints (where old fronds were)
    float segments = step(0.85, fract(p.y * 12.0 + coarseNoise * 0.3));
    bark = bark * (1.0 - segments * 0.4);

    // Trunk color - earthy browns
    vec3 trunkDark = vec3(0.18, 0.10, 0.04);
    vec3 trunkLight = vec3(0.42, 0.28, 0.14);
    vec3 trunkColor = mix(trunkDark, trunkLight, bark);

    // === FRONDS - 5 leaves, all from crown ===
    float frondShape = 0.0;
    vec2 crown = trunkTop + vec2(sway * 0.4, 0.0);

    vec3 neonCyan = vec3(0.2, 0.85, 0.92);

    // Scale up leaves - bigger tree
    float scale = 1.25;

    // Leaf 1: Top-left "ear" - goes up-left then curves down
    vec2 l1_ctrl = crown + vec2(-0.12 + sway, 0.16) * scale;
    vec2 l1_end = crown + vec2(-0.24 + sway * 2.0, 0.02) * scale;
    float leaf1 = palmLeaf(p, crown, l1_ctrl, l1_end, 0.052 * scale);
    frondShape = max(frondShape, smoothstep(0.003, -0.003, leaf1));

    // Leaf 2: Top-right "ear" - goes up-right then curves down
    vec2 l2_ctrl = crown + vec2(0.11 + sway, 0.15) * scale;
    vec2 l2_end = crown + vec2(0.22 + sway * 2.0, 0.0) * scale;
    float leaf2 = palmLeaf(p, crown, l2_ctrl, l2_end, 0.048 * scale);
    frondShape = max(frondShape, smoothstep(0.003, -0.003, leaf2));

    // Leaf 3: Center top - goes straight up
    vec2 l3_ctrl = crown + vec2(sway * 0.5, 0.12) * scale;
    vec2 l3_end = crown + vec2(sway, 0.20) * scale;
    float leaf3 = palmLeaf(p, crown, l3_ctrl, l3_end, 0.040 * scale);
    frondShape = max(frondShape, smoothstep(0.003, -0.003, leaf3));

    // Leaf 4: Left drooping - curves out-up first (convex top), then droops (concave bottom)
    vec2 l4_ctrl = crown + vec2(-0.14 + sway, 0.04) * scale;
    vec2 l4_end = crown + vec2(-0.20 + sway * 2.0, -0.16) * scale;
    float leaf4 = palmLeaf(p, crown, l4_ctrl, l4_end, 0.046 * scale);
    frondShape = max(frondShape, smoothstep(0.003, -0.003, leaf4));

    // Leaf 5: Right drooping - curves out-up first (convex top), then droops (concave bottom)
    vec2 l5_ctrl = crown + vec2(0.13 + sway, 0.03) * scale;
    vec2 l5_end = crown + vec2(0.18 + sway * 2.0, -0.17) * scale;
    float leaf5 = palmLeaf(p, crown, l5_ctrl, l5_end, 0.044 * scale);
    frondShape = max(frondShape, smoothstep(0.003, -0.003, leaf5));

    // === COMBINE ===
    vec3 finalColor = trunkColor;
    float finalAlpha = inTrunk * 0.9;

    finalColor = mix(finalColor, neonCyan, frondShape);
    finalAlpha = max(finalAlpha, frondShape);

    // Subtle glow
    float glow = 0.0;
    glow = max(glow, smoothstep(0.06, 0.0, leaf1) * 0.1);
    glow = max(glow, smoothstep(0.06, 0.0, leaf2) * 0.1);
    glow = max(glow, smoothstep(0.06, 0.0, leaf3) * 0.1);
    glow = max(glow, smoothstep(0.06, 0.0, leaf4) * 0.1);
    glow = max(glow, smoothstep(0.06, 0.0, leaf5) * 0.1);

    finalColor += neonCyan * glow;
    finalAlpha = max(finalAlpha, glow * 0.4);

    finalAlpha *= u_opacity;
    gl_FragColor = vec4(finalColor, finalAlpha);
}
`;

// Export all shaders as a map for easy access
export const fragmentShaders: Record<string, string> = {
    'dot-grid': dotGridShader,
    'scanlines': scanlinesShader,
    'crosshatch': crosshatchShader,
    'ascii-noise': asciiNoiseShader,
    'vertical-lines': verticalLinesShader,
    'binary-rain': binaryRainShader,
    'neon-palm': neonPalmShader,
};
