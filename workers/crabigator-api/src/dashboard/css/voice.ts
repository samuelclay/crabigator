// Voice button and waveform bar styles
export const voiceCss = `
/* Voice button - .input-area prefix needed to override .input-area button styles */
.input-area .voice-btn {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 50%;
    width: 38px;
    height: 38px;
    min-width: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
    padding: 0;
    color: var(--text-mid);
    font-size: 0;
    letter-spacing: 0;
    text-transform: none;
    font-weight: normal;
}
.input-area .voice-btn svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
}
.input-area .voice-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    box-shadow: 0 0 12px var(--glow-cyan);
    transform: none;
}

/* Recording state - pulsing red/orange ring */
.input-area .voice-btn.recording {
    border-color: #f97316;
    animation: voice-pulse 1.5s ease-in-out infinite;
}
@keyframes voice-pulse {
    0%, 100% { box-shadow: 0 0 0 0 rgba(249, 115, 22, 0.4); }
    50% { box-shadow: 0 0 0 6px rgba(249, 115, 22, 0); }
}

/* Transcribing state - spinning cyan border */
.input-area .voice-btn.transcribing {
    border-color: var(--accent-cyan);
    animation: voice-spin 1s linear infinite;
    border-style: dashed;
}
@keyframes voice-spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
}
.input-area .voice-btn.transcribing svg {
    animation: voice-spin 1s linear infinite reverse;
}

/* Error flash */
.input-area .voice-btn.voice-error {
    border-color: #f87171;
    color: #f87171;
}

/* Waveform bars */
.voice-bars {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 20px;
}
.voice-bar {
    width: 3px;
    height: 4px;
    background: var(--accent-cyan);
    border-radius: 1px;
    transition: height 0.05s ease;
}
`;
