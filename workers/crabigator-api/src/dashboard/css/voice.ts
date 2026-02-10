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

/* Cancel button - replaces voice+keyboard during recording */
.input-area .voice-cancel-btn {
    background: transparent;
    border: 1px solid var(--border-dim);
    border-radius: 50%;
    width: 38px;
    height: 38px;
    min-width: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: border-color 0.2s, color 0.2s, box-shadow 0.2s;
    padding: 0;
    color: var(--text-dim);
    font-size: 0;
    letter-spacing: 0;
    text-transform: none;
    font-weight: normal;
}
.input-area .voice-cancel-btn svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
}
.input-area .voice-cancel-btn:hover {
    border-color: #f97316;
    color: #f97316;
    box-shadow: 0 0 12px rgba(249, 115, 22, 0.3);
    transform: none;
}
.input-area .voice-cancel-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}

/* Voice action buttons container */
.voice-actions {
    display: flex;
    gap: 8px;
    align-items: center;
}

/* Edit button - ghost/outline style */
.input-area .voice-edit-btn {
    background: transparent;
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 10px 16px;
    color: var(--text-mid);
    cursor: pointer;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    transition: border-color 0.2s, color 0.2s, box-shadow 0.2s;
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
}
.input-area .voice-edit-btn svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
}
.input-area .voice-edit-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    box-shadow: 0 0 12px var(--glow-cyan);
    transform: none;
}
.input-area .voice-edit-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
    border-color: var(--border-dim);
    color: var(--text-dim);
}

/* Voice send button - primary gradient */
.input-area .voice-send-btn {
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 8px;
    padding: 10px 20px;
    color: var(--bg-abyss);
    font-family: 'JetBrains Mono', monospace;
    font-weight: 700;
    font-size: 12px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: box-shadow 0.2s;
    white-space: nowrap;
}
.input-area .voice-send-btn:hover {
    box-shadow: 0 8px 24px var(--glow-cyan);
    transform: none;
}
.input-area .voice-send-btn:disabled {
    background: var(--bg-surface);
    color: var(--text-dim);
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
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
/* Waveform bars inside overlay - push to right */
.voice-overlay .voice-bars {
    margin-left: auto;
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
    .input-area .voice-cancel-btn {
        width: 32px;
        height: 32px;
        min-width: 32px;
    }
    .input-area .voice-cancel-btn svg {
        width: 14px;
        height: 14px;
    }
    .input-area .voice-edit-btn {
        padding: 8px 10px;
        font-size: 11px;
    }
    .input-area .voice-send-btn {
        padding: 8px 12px;
        font-size: 11px;
    }
}
`;
