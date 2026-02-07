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
}

/* Voice overlay - replaces input during recording/uploading */
.voice-overlay {
    flex: 1;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 10px 14px;
    display: none;
    align-items: center;
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--text-bright);
    position: relative;
    overflow: hidden;
}
.voice-overlay.recording {
    border-color: #f97316;
}
.voice-overlay.uploading {
    border-color: var(--accent-cyan);
}

/* Recording dot */
.voice-rec-dot {
    width: 8px;
    height: 8px;
    background: #f87171;
    border-radius: 50%;
    margin-right: 10px;
    flex-shrink: 0;
    animation: voice-rec-pulse 1.5s ease-in-out infinite;
}
@keyframes voice-rec-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
}

/* Timer text */
.voice-timer-elapsed {
    color: var(--text-bright);
}
.voice-timer-sep, .voice-timer-max {
    color: var(--text-dim);
}

/* Upload progress bar */
.voice-progress-fill {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    background: linear-gradient(135deg, rgba(34, 211, 238, 0.15), rgba(59, 130, 246, 0.15));
    border-radius: 8px;
    transition: width 0.3s ease;
}
.voice-progress-label {
    position: relative;
    z-index: 1;
    color: var(--text-mid);
}

@media (max-width: 768px) {
    .voice-overlay {
        padding: 8px 10px;
        font-size: 12px;
    }
}
`;
