// WebGL texture rendering JavaScript for landing page
import {
    vertexShader,
    dotGridShader,
    scanlinesShader,
    crosshatchShader,
    asciiNoiseShader,
    verticalLinesShader,
    binaryRainShader,
} from './shaders';

export const webglJs = `
// WebGL Terminal Texture System
(function() {
    const canvas = document.getElementById('texture-canvas');
    if (!canvas) return;

    const gl = canvas.getContext('webgl', { alpha: true, premultipliedAlpha: false });
    if (!gl) {
        console.log('WebGL not supported, using CSS fallback');
        canvas.style.display = 'none';
        return;
    }

    // Shader sources (inlined for Cloudflare Workers)
    const vertexShaderSrc = ${JSON.stringify(vertexShader)};

    const fragmentShaders = {
        'cta-section': ${JSON.stringify(dotGridShader)},
        'showcase': ${JSON.stringify(scanlinesShader)},
        'interactive': ${JSON.stringify(crosshatchShader)},
        'pricing': ${JSON.stringify(asciiNoiseShader)},
        'open-source': ${JSON.stringify(verticalLinesShader)},
        'mobile-apps': ${JSON.stringify(binaryRainShader)},
    };

    // Compile shader
    function compileShader(type, source) {
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            console.error('Shader compile error:', gl.getShaderInfoLog(shader));
            gl.deleteShader(shader);
            return null;
        }
        return shader;
    }

    // Create program
    function createProgram(vertSrc, fragSrc) {
        const vertShader = compileShader(gl.VERTEX_SHADER, vertSrc);
        const fragShader = compileShader(gl.FRAGMENT_SHADER, fragSrc);
        if (!vertShader || !fragShader) return null;

        const program = gl.createProgram();
        gl.attachShader(program, vertShader);
        gl.attachShader(program, fragShader);
        gl.linkProgram(program);

        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            console.error('Program link error:', gl.getProgramInfoLog(program));
            return null;
        }

        return program;
    }

    // Create all shader programs
    const programs = {};
    for (const [section, fragSrc] of Object.entries(fragmentShaders)) {
        programs[section] = createProgram(vertexShaderSrc, fragSrc);
    }

    // Setup geometry (fullscreen quad)
    const positionBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
        -1, -1, 1, -1, -1, 1,
        -1, 1, 1, -1, 1, 1,
    ]), gl.STATIC_DRAW);

    // Current active section - start with dot-grid always visible for testing
    let activeSection = 'cta-section';
    let startTime = Date.now();

    // Resize canvas
    function resize() {
        const dpr = window.devicePixelRatio || 1;
        canvas.width = window.innerWidth * dpr;
        canvas.height = window.innerHeight * dpr;
        gl.viewport(0, 0, canvas.width, canvas.height);
    }
    resize();
    window.addEventListener('resize', resize);

    // Render function
    function render() {
        if (!activeSection || !programs[activeSection]) {
            gl.clearColor(0, 0, 0, 0);
            gl.clear(gl.COLOR_BUFFER_BIT);
            requestAnimationFrame(render);
            return;
        }

        const program = programs[activeSection];
        gl.useProgram(program);

        // Setup attribute
        const posLoc = gl.getAttribLocation(program, 'a_position');
        gl.enableVertexAttribArray(posLoc);
        gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
        gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);

        // Setup uniforms
        const resLoc = gl.getUniformLocation(program, 'u_resolution');
        const timeLoc = gl.getUniformLocation(program, 'u_time');
        const opacityLoc = gl.getUniformLocation(program, 'u_opacity');
        const colorLoc = gl.getUniformLocation(program, 'u_color');

        gl.uniform2f(resLoc, canvas.width, canvas.height);
        gl.uniform1f(timeLoc, (Date.now() - startTime) / 1000.0);
        gl.uniform1f(opacityLoc, 1.0); // Full opacity for testing
        gl.uniform3f(colorLoc, 0.133, 0.827, 0.933); // Cyan accent

        // Disable blending for testing - solid colors
        gl.disable(gl.BLEND);

        // Clear and draw
        gl.clearColor(0, 0, 0, 0);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.drawArrays(gl.TRIANGLES, 0, 6);

        requestAnimationFrame(render);
    }

    // Intersection Observer to detect which section is visible
    const sections = document.querySelectorAll('.cta-section, .showcase, .interactive, .pricing, .open-source, .mobile-apps');

    const observer = new IntersectionObserver((entries) => {
        let maxRatio = 0;
        let maxSection = null;

        entries.forEach(entry => {
            if (entry.intersectionRatio > maxRatio) {
                maxRatio = entry.intersectionRatio;
                maxSection = entry.target.className.split(' ').find(c =>
                    ['cta-section', 'showcase', 'interactive', 'pricing', 'open-source', 'mobile-apps'].includes(c)
                );
            }
        });

        // Only switch if we have significant visibility
        if (maxRatio > 0.3 && maxSection) {
            activeSection = maxSection;
        } else if (maxRatio < 0.1) {
            activeSection = null;
        }
    }, {
        threshold: [0, 0.1, 0.2, 0.3, 0.5, 0.7, 1.0]
    });

    sections.forEach(section => observer.observe(section));

    // Start rendering
    render();
})();
`;
