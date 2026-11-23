// Paint the pixel coast once at each size. A small transparent WebGL layer
// animates the water and its reflections beneath the revolving lamp.
const waterShader = `
precision mediump float;
uniform vec2 size, lantern, moon;
uniform float horizon, seconds, light, bearing;
uniform vec3 beacon;
float hash(vec2 p) { return fract(sin(dot(mod(p,256.),vec2(12.1,38.7)))*431.8); }
void main() {
    vec2 p = vec2(gl_FragCoord.x, size.y-gl_FragCoord.y);
    float depth = (p.y-horizon)/max(1.,size.y-horizon);
    vec3 color = vec3(0.76,0.84,0.89);
    float alpha = 0.;
    if (depth > 0.) {
        float stretch = 1.5+8.*depth;
        float y = floor(p.y/2.);
        float x = p.x/stretch+sin(y*1.7+seconds*0.65);
        float glint = smoothstep(.70,.97,hash(vec2(floor(x),y))) * step(mod(p.y,3.),1.0);
        float wobble = 5.*sin(y*1.3+seconds);
        float silver = exp(-abs(p.x-moon.x+wobble)/(4.+depth*32.));
        float reflected = exp(-abs(p.x-lantern.x-depth*bearing*size.x*.28+wobble)/(3.+depth*22.))*light;
        color = mix(color,beacon,reflected/(silver+reflected+.001));
        alpha = glint*(.13+.7*silver+.95*reflected);
    }
    gl_FragColor = vec4(color, clamp(alpha,0.,1.));
}`;

export const harborArtJs = `
const art = scene.querySelector('.harbor-art'), water = scene.querySelector('.harbor-water');
const lamp = scene.querySelector('.harbor-lamp');
const ctx = art.getContext('2d');
let gl, uniforms, width = 0, height = 0, lanternX, lanternY, moonX, sea;
function prepareWater() {
    gl = water.getContext('webgl', { alpha: true, premultipliedAlpha: false, antialias: false, depth: false });
    if (!gl) return;
    const program = gl.createProgram();
    for (const [type, source] of [
        [gl.VERTEX_SHADER, 'attribute vec2 point;void main(){gl_Position=vec4(point,0.,1.);}'],
        [gl.FRAGMENT_SHADER, ${JSON.stringify(waterShader)}]
    ]) {
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        gl.attachShader(program, shader);
        gl.deleteShader(shader);
    }
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) { gl.deleteProgram(program); return; }
    gl.useProgram(program);
    const points = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, points);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const pos = gl.getAttribLocation(program, 'point');
    gl.enableVertexAttribArray(pos); gl.vertexAttribPointer(pos, 2, gl.FLOAT, false, 0, 0);
    uniforms = Object.fromEntries(['size', 'lantern', 'moon', 'horizon', 'seconds', 'light', 'bearing', 'beacon'].map(name => [name, gl.getUniformLocation(program, name)]));
}
water.addEventListener('webglcontextlost', event => { event.preventDefault(); uniforms = null; });
water.addEventListener('webglcontextrestored', () => { prepareWater(); paint(); });
function sizeHarbor(w, h) {
    if (!ctx || !w || !h) return;
    const px = Math.max(2, w / 590);
    const nextWidth = Math.round(w / px), nextHeight = Math.round(h / px);
    if (width === nextWidth && height === nextHeight) return;
    width = art.width = water.width = nextWidth;
    height = art.height = water.height = nextHeight;
    if (gl === undefined) prepareWater();
    let seed = 812;
    const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; };
    const rect = (x, y, width, height, fill) => {
        ctx.fillStyle = fill;
        ctx.fillRect(Math.round(x), Math.round(y), Math.ceil(width), Math.ceil(height));
    };
    const narrow = w < 605;
    sea = Math.floor(height * (narrow ? .79 : .71));
    moonX = Math.floor(width * (narrow ? .76 : .61));
    const moonY = Math.round(height * (narrow ? .42 : .15));
    const sky = ctx.createLinearGradient(0, 0, 0, sea);
    sky.addColorStop(0, '#18263d');
    sky.addColorStop(.48, '#40506b');
    sky.addColorStop(.8, '#a27b84');
    sky.addColorStop(1, '#f0b399');
    rect(0, 0, width, sea, sky);
    for (let i = 0; i < width * .8; i++) {
        const x = random() * width, y = random() * sea * .82;
        rect(x, y, 1, 1, random() > .72 ? '#f5d4b1' : '#a2a4b4');
        if (i % 47 === 0) {
            rect(x - 1, y, 3, 1, '#efd5ba');
            rect(x, y - 1, 1, 3, '#efd5ba');
        }
    }
    // Draw the moon one scanline at a time to keep the pixel silhouette.
    for (let y = -9; y <= 9; y++) {
        const r = Math.floor(Math.sqrt(82 - y * y));
        rect(moonX - r, moonY + y, 2 * r + 1, 1, '#ffdeb0');
    }
    for (let i = 0; i < 36; i++) {
        const y = sea * (.38 + random() * .53), x = random() * width;
        const len = 4 + random() * width * .08;
        rect(x, y, len, 1, '#c4a0a2');
        rect(x + 3, y - 1, len * .62, 1, '#a28b97');
    }
    const ridge = (x, phase) => Math.sin(x * 12 + phase) * .37 + Math.sin(x * 27 + phase) * .17 + Math.sin(x * 5 - phase) * .42;
    [['#76748a', .77, .14], ['#54576d', .84, .13], ['#3c465b', .92, .11]].forEach(([color, base, amplitude], layer) => {
        for (let x = 0; x < width; x++) {
            const y = sea * (base + amplitude * ridge(x / width, layer * 2));
            rect(x, y, 1, sea - y, color);
        }
    });
    const ocean = ctx.createLinearGradient(0, sea, 0, height);
    ocean.addColorStop(0, '#526679');
    ocean.addColorStop(.22, '#354e62');
    ocean.addColorStop(1, '#142c3b');
    rect(0, sea, width, height - sea, ocean);
    for (let i = 0; i < width * 1.4; i++) {
        const x = random() * width, y = sea + random() * (height - sea);
        rect(x, y, 1 + random() * 5, 1, '#82959a45');
    }
    lanternX = Math.floor(width * (narrow ? .19 : .545));
    const foot = sea + Math.round(height * .055);
    const unit = Math.max(1, Math.round(height / 155));
    const ly = lanternY = foot - 27 * unit, lx = lanternX;
    // A rocky spit, a keeper's cottage and a striped harbor light.
    for (let x = -30 * unit; x < 35 * unit; x++) {
        const fall = Math.max(0, 1 - Math.abs(x) / (34 * unit));
        const y = foot - 2 * unit + Math.round((1 - fall) * 5 * unit + Math.sin(x * .55));
        rect(lx + x, y, 1, Math.max(1, fall * 6 * unit), '#243443');
    }
    for (let j = 0; j < 18 * unit; j++) {
        const half = Math.round(2 * unit + j * .15);
        rect(lx - half, ly + 5 * unit + j, half * 2 + 1, 1, j > 6 * unit && j < 11 * unit ? '#bb7666' : '#dfcfb9');
        rect(lx + half - 1, ly + 5 * unit + j, 2, 1, '#998f90');
    }
    rect(lx - 4 * unit, ly - 2 * unit, 8 * unit, 5 * unit, '#3b4652');
    rect(lx - 2 * unit, ly - unit, 4 * unit, 3 * unit, '#8b99a1');
    rect(lx - 5 * unit, ly + 3 * unit, 10 * unit, 2 * unit, '#333b4b');
    for (let j = 0; j < 4 * unit; j++) {
        rect(lx - j, ly - 6 * unit + j, 2 * j + 1, 1, '#553d47');
    }
    rect(lx - 18 * unit, foot - 7 * unit, 11 * unit, 6 * unit, '#baa69d');
    rect(lx - 19 * unit, foot - 9 * unit, 13 * unit, 3 * unit, '#744f57');
    rect(lx - 15 * unit, foot - 5 * unit, 2 * unit, 2 * unit, '#ffe1a2');
    // Near shore at the foot of the scene.
    for (let x = 0; x < width; x++) {
        const y = height * (1.035 - .13 * Math.pow(Math.max(0, 1 - x / (width * .28)), 2)) + ridge(x / width, 4) * 3;
        rect(x, y, 1, height - y, '#101f2b');
    }
    lamp.style.setProperty('--beam-length', w * .48 + 'px');
    lamp.style.setProperty('--beam-width', w * .09 + 'px');
    lamp.style.left = 100 * lx / width + '%';
    lamp.style.top = 100 * ly / height + '%';
}
function drawHarbor(t, light, rgb) {
    // Project a horizontal turn onto the tilted view of the harbor.
    const angle = t * Math.PI / 5;
    const bearing = Math.cos(angle);
    lamp.style.setProperty('--sweep', 'matrix(' + bearing + ',' + (.16 * Math.sin(angle) - .03) + ',0,1,0,0)');

    lamp.style.opacity = String(light);
    if (!uniforms || gl.isContextLost()) return;
    gl.viewport(0, 0, width, height);
    gl.uniform2f(uniforms.size, width, height); gl.uniform2f(uniforms.lantern, lanternX, lanternY);
    gl.uniform2f(uniforms.moon, moonX, 0); gl.uniform1f(uniforms.horizon, sea);
    gl.uniform1f(uniforms.seconds, t % (Math.PI * 40)); gl.uniform1f(uniforms.light, light);
    gl.uniform1f(uniforms.bearing, bearing); gl.uniform3f(uniforms.beacon, rgb[0]/255, rgb[1]/255, rgb[2]/255);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
}
`;
