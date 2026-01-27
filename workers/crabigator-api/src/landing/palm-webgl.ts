// WebGL initialization for the neon palm tree in the interactive section
import { vertexShader, neonPalmShader } from './shaders';

export const palmWebglJs = `
(function() {
    // Find the interactive section and text area
    const section = document.querySelector('.interactive');
    const textArea = document.querySelector('.interactive-text');
    if (!section) return;

    // On mobile (<=1024px), canvas goes in text area only; on desktop, whole section
    const isMobile = () => window.innerWidth <= 1024;

    // Create canvas element
    const canvas = document.createElement('canvas');
    canvas.id = 'palm-canvas';
    canvas.style.cssText = 'position: absolute; top: 0; left: 0; width: 100%; height: 100%; z-index: 1; pointer-events: none;';

    // Get current container based on viewport
    function getContainer() {
        return (isMobile() && textArea) ? textArea : section;
    }

    // Insert canvas into appropriate container
    let currentContainer = getContainer();
    currentContainer.insertBefore(canvas, currentContainer.firstChild);

    const gl = canvas.getContext('webgl', { alpha: true, preserveDrawingBuffer: true });
    if (!gl) {
        console.log('WebGL not supported for palm tree');
        canvas.style.display = 'none';
        return;
    }

    // Shader sources
    const vertexShaderSrc = ${JSON.stringify(vertexShader)};
    const fragmentShaderSrc = ${JSON.stringify(neonPalmShader)};

    // Compile shader
    function compileShader(type, source) {
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            console.error('Palm shader compile error:', gl.getShaderInfoLog(shader));
            gl.deleteShader(shader);
            return null;
        }
        return shader;
    }

    // Create program
    const vertShader = compileShader(gl.VERTEX_SHADER, vertexShaderSrc);
    const fragShader = compileShader(gl.FRAGMENT_SHADER, fragmentShaderSrc);
    if (!vertShader || !fragShader) return;

    const program = gl.createProgram();
    gl.attachShader(program, vertShader);
    gl.attachShader(program, fragShader);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
        console.error('Palm program link error:', gl.getProgramInfoLog(program));
        return;
    }

    // Setup geometry (fullscreen quad)
    const positionBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
        -1, -1, 1, -1, -1, 1,
        -1, 1, 1, -1, 1, 1,
    ]), gl.STATIC_DRAW);

    const startTime = Date.now();
    let animationId = null;
    let isVisible = false;

    // Resize canvas to match container (and re-parent if needed)
    function resize() {
        const newContainer = getContainer();
        if (newContainer !== currentContainer) {
            currentContainer = newContainer;
            currentContainer.insertBefore(canvas, currentContainer.firstChild);
        }
        const rect = currentContainer.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
    }

    // Render function
    function render() {
        if (!isVisible) {
            animationId = null;
            return;
        }

        resize();
        gl.viewport(0, 0, canvas.width, canvas.height);

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
        gl.uniform1f(opacityLoc, 0.35); // 35% opacity - visible but subtle
        gl.uniform3f(colorLoc, 0.133, 0.827, 0.933); // Cyan accent

        // Enable blending for transparency
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

        // Clear and draw
        gl.clearColor(0, 0, 0, 0);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.drawArrays(gl.TRIANGLES, 0, 6);

        animationId = requestAnimationFrame(render);
    }

    // Intersection Observer to only render when visible
    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            isVisible = entry.isIntersecting;
            if (isVisible && !animationId) {
                render();
            }
        });
    }, { threshold: 0.1 });

    observer.observe(currentContainer);

    // Initial resize
    resize();
    window.addEventListener('resize', resize);
})();
`;
