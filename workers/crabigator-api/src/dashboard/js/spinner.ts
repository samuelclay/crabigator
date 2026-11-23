export const spinnerJs = `
        // Spinner frames for thinking indicator
        const SPINNER_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let spinnerFrameIndex = 0;

        // Start the global spinner animation
        setInterval(() => {
            const spinners = document.querySelectorAll('.thinking-spinner');
            if (spinners.length === 0) return;
            spinnerFrameIndex = (spinnerFrameIndex + 1) % SPINNER_FRAMES.length;
            const frame = SPINNER_FRAMES[spinnerFrameIndex];
            spinners.forEach(el => {
                el.textContent = frame;
            });
        }, 80);
`;
