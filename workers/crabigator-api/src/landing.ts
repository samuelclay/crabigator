// Landing page HTML served at /
import { landingCss } from './landing/css';
import { landingJs } from './landing/js';

export const landingHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator - control Claude Code from anywhere</title>
    <meta name="description" content="Answer permissions, approve plans, and respond to questions from your phone. Real-time monitoring and remote control for Claude Code sessions.">
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 512 512'><path fill='%23ea580c' d='M379.204,266.236c-1.879,0-3.78-0.07-5.648-0.206c-11.05-0.806-19.356-10.418-18.55-21.468c0.807-11.05,10.416-19.352,21.469-18.55c0.9,0.066,1.819,0.099,2.729,0.099c20.173,0,36.585-16.412,36.585-36.585c0-1.337-0.072-2.687-0.215-4.01c-1.193-11.016,6.77-20.913,17.786-22.106c11.014-1.19,20.913,6.77,22.105,17.786c0.298,2.751,0.449,5.553,0.449,8.329C455.914,231.824,421.502,266.236,379.204,266.236z'/><path fill='%23f97316' d='M415.008,97.65v-53.37c4.962-1.106,10.113-1.709,15.408-1.709c38.929,0,70.489,31.558,70.489,70.487s-31.558,70.487-70.489,70.487c-38.929,0-70.487-31.558-70.487-70.487c0-5.295,0.603-10.446,1.709-15.408L415.008,97.65L415.008,97.65z'/><path fill='%23ea580c' d='M491.941,331.928c-5.383,0-10.752-2.153-14.704-6.409c-20.311-21.868-49.921-36.124-83.371-40.14c-11.001-1.321-18.849-11.31-17.528-22.311c1.321-11.001,11.302-18.851,22.311-17.528c42.871,5.148,81.222,23.853,107.989,52.672c7.541,8.119,7.073,20.813-1.046,28.354C501.726,330.153,496.828,331.928,491.941,331.928z'/><path fill='%23ea580c' d='M482.529,411.418c-6.567,0-13-3.219-16.844-9.136c-16.534-25.457-43.78-44.737-76.72-54.289c-10.642-3.086-16.767-14.215-13.68-24.856c3.086-10.642,14.212-16.766,24.856-13.681c42.186,12.234,77.414,37.438,99.195,70.969c6.035,9.292,3.396,21.718-5.897,27.753C490.063,410.369,486.274,411.418,482.529,411.418z'/><path fill='%23ea580c' d='M434.652,469.431c-7.463,0-14.63-4.182-18.087-11.357c-13.569-28.15-40.168-51.891-72.98-65.132c-10.275-4.146-15.242-15.837-11.096-26.112c4.148-10.276,15.839-15.243,26.113-11.096c42.575,17.181,75.996,47.339,94.109,84.92c4.811,9.982,0.619,21.972-9.362,26.783C440.54,468.79,437.573,469.431,434.652,469.431z'/><path fill='%23f97316' d='M132.796,266.236c-42.298,0-76.709-34.412-76.709-76.709c0-2.775,0.151-5.577,0.449-8.329c1.193-11.016,11.097-18.972,22.105-17.786c11.016,1.193,18.979,11.091,17.786,22.106c-0.143,1.323-0.215,2.672-0.215,4.01c0,20.172,16.412,36.585,36.586,36.585c0.91,0,1.828-0.033,2.729-0.099c11.062-0.804,20.663,7.499,21.469,18.55c0.805,11.05-7.499,20.663-18.55,21.468C136.575,266.167,134.675,266.236,132.796,266.236z'/><path fill='%23f97316' d='M20.058,331.928c-4.887,0-9.785-1.775-13.649-5.363c-8.119-7.541-8.587-20.235-1.046-28.354c26.769-28.819,65.12-47.524,107.989-52.672c11.009-1.317,20.989,6.527,22.311,17.528c1.321,11.001-6.527,20.991-17.528,22.311c-33.451,4.017-63.06,18.272-83.373,40.141C30.81,329.775,25.44,331.928,20.058,331.928z'/><path fill='%23f97316' d='M29.47,411.418c-3.746,0-7.534-1.047-10.909-3.239c-9.293-6.035-11.932-18.461-5.897-27.753c21.78-33.531,57.008-58.736,99.195-70.969c10.644-3.087,21.77,3.039,24.856,13.681c3.087,10.641-3.039,21.77-13.681,24.856c-32.938,9.552-60.186,28.832-76.72,54.289C42.472,408.197,36.036,411.418,29.47,411.418z'/><path fill='%23f97316' d='M77.347,469.431c-2.922,0-5.888-0.641-8.696-1.994c-9.982-4.811-14.173-16.803-9.362-26.783c18.112-37.58,51.534-67.739,94.109-84.92c10.277-4.146,21.967,0.821,26.113,11.096c4.146,10.275-0.821,21.966-11.096,26.112c-32.813,13.241-59.412,36.982-72.98,65.132C91.978,465.247,84.81,469.431,77.347,469.431z'/><ellipse fill='%23fb923c' cx='255.996' cy='294.45' rx='144.436' ry='120.976'/><path fill='%23fb923c' d='M96.991,97.65v-53.37c-4.962-1.106-10.113-1.709-15.408-1.709c-38.929,0-70.487,31.558-70.487,70.487s31.558,70.487,70.487,70.487s70.487-31.558,70.487-70.487c0-5.295-0.602-10.446-1.709-15.408L96.991,97.65L96.991,97.65z'/><path fill='%23f97316' d='M400.43,294.451c0-66.813-64.664-120.975-144.431-120.976v241.952C335.767,415.428,400.43,361.265,400.43,294.451z'/></svg>">
    <style>${landingCss}</style>
</head>
<body>
    <!-- Navigation -->
    <nav class="nav">
        <a href="/" class="nav-logo">
            <svg viewBox="0 0 512 512"><path fill="#ea580c" d="M379.204,266.236c-1.879,0-3.78-0.07-5.648-0.206c-11.05-0.806-19.356-10.418-18.55-21.468c0.807-11.05,10.416-19.352,21.469-18.55c0.9,0.066,1.819,0.099,2.729,0.099c20.173,0,36.585-16.412,36.585-36.585c0-1.337-0.072-2.687-0.215-4.01c-1.193-11.016,6.77-20.913,17.786-22.106c11.014-1.19,20.913,6.77,22.105,17.786c0.298,2.751,0.449,5.553,0.449,8.329C455.914,231.824,421.502,266.236,379.204,266.236z"/><path fill="#f97316" d="M415.008,97.65v-53.37c4.962-1.106,10.113-1.709,15.408-1.709c38.929,0,70.489,31.558,70.489,70.487s-31.558,70.487-70.489,70.487c-38.929,0-70.487-31.558-70.487-70.487c0-5.295,0.603-10.446,1.709-15.408L415.008,97.65L415.008,97.65z"/><path fill="#ea580c" d="M491.941,331.928c-5.383,0-10.752-2.153-14.704-6.409c-20.311-21.868-49.921-36.124-83.371-40.14c-11.001-1.321-18.849-11.31-17.528-22.311c1.321-11.001,11.302-18.851,22.311-17.528c42.871,5.148,81.222,23.853,107.989,52.672c7.541,8.119,7.073,20.813-1.046,28.354C501.726,330.153,496.828,331.928,491.941,331.928z"/><path fill="#ea580c" d="M482.529,411.418c-6.567,0-13-3.219-16.844-9.136c-16.534-25.457-43.78-44.737-76.72-54.289c-10.642-3.086-16.767-14.215-13.68-24.856c3.086-10.642,14.212-16.766,24.856-13.681c42.186,12.234,77.414,37.438,99.195,70.969c6.035,9.292,3.396,21.718-5.897,27.753C490.063,410.369,486.274,411.418,482.529,411.418z"/><path fill="#ea580c" d="M434.652,469.431c-7.463,0-14.63-4.182-18.087-11.357c-13.569-28.15-40.168-51.891-72.98-65.132c-10.275-4.146-15.242-15.837-11.096-26.112c4.148-10.276,15.839-15.243,26.113-11.096c42.575,17.181,75.996,47.339,94.109,84.92c4.811,9.982,0.619,21.972-9.362,26.783C440.54,468.79,437.573,469.431,434.652,469.431z"/><path fill="#f97316" d="M132.796,266.236c-42.298,0-76.709-34.412-76.709-76.709c0-2.775,0.151-5.577,0.449-8.329c1.193-11.016,11.097-18.972,22.105-17.786c11.016,1.193,18.979,11.091,17.786,22.106c-0.143,1.323-0.215,2.672-0.215,4.01c0,20.172,16.412,36.585,36.586,36.585c0.91,0,1.828-0.033,2.729-0.099c11.062-0.804,20.663,7.499,21.469,18.55c0.805,11.05-7.499,20.663-18.55,21.468C136.575,266.167,134.675,266.236,132.796,266.236z"/><path fill="#f97316" d="M20.058,331.928c-4.887,0-9.785-1.775-13.649-5.363c-8.119-7.541-8.587-20.235-1.046-28.354c26.769-28.819,65.12-47.524,107.989-52.672c11.009-1.317,20.989,6.527,22.311,17.528c1.321,11.001-6.527,20.991-17.528,22.311c-33.451,4.017-63.06,18.272-83.373,40.141C30.81,329.775,25.44,331.928,20.058,331.928z"/><path fill="#f97316" d="M29.47,411.418c-3.746,0-7.534-1.047-10.909-3.239c-9.293-6.035-11.932-18.461-5.897-27.753c21.78-33.531,57.008-58.736,99.195-70.969c10.644-3.087,21.77,3.039,24.856,13.681c3.087,10.641-3.039,21.77-13.681,24.856c-32.938,9.552-60.186,28.832-76.72,54.289C42.472,408.197,36.036,411.418,29.47,411.418z"/><path fill="#f97316" d="M77.347,469.431c-2.922,0-5.888-0.641-8.696-1.994c-9.982-4.811-14.173-16.803-9.362-26.783c18.112-37.58,51.534-67.739,94.109-84.92c10.277-4.146,21.967,0.821,26.113,11.096c4.146,10.275-0.821,21.966-11.096,26.112c-32.813,13.241-59.412,36.982-72.98,65.132C91.978,465.247,84.81,469.431,77.347,469.431z"/><ellipse fill="#fb923c" cx="255.996" cy="294.45" rx="144.436" ry="120.976"/><path fill="#fb923c" d="M96.991,97.65v-53.37c-4.962-1.106-10.113-1.709-15.408-1.709c-38.929,0-70.487,31.558-70.487,70.487s31.558,70.487,70.487,70.487s70.487-31.558,70.487-70.487c0-5.295-0.602-10.446-1.709-15.408L96.991,97.65L96.991,97.65z"/><path fill="#f97316" d="M400.43,294.451c0-66.813-64.664-120.975-144.431-120.976v241.952C335.767,415.428,400.43,361.265,400.43,294.451z"/></svg>
            Crabigator
        </a>
        <div class="nav-links">
            <a href="#features" class="nav-link">Features</a>
            <a href="#pricing" class="nav-link">Pricing</a>
            <a href="#install" class="nav-link">Install</a>
            <a href="/dashboard" class="nav-btn">Open Dashboard</a>
            <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="nav-github">
                <svg viewBox="0 0 16 16" fill="currentColor">
                    <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
                </svg>
            </a>
        </div>
    </nav>

    <!-- Hero Section -->
    <section class="hero">
        <div class="hero-content">
            <div class="hero-text">
                <p class="hero-tagline"><svg class="icon" viewBox="0 0 256 256"><path d="M176,16H80A24,24,0,0,0,56,40V216a24,24,0,0,0,24,24h96a24,24,0,0,0,24-24V40A24,24,0,0,0,176,16ZM140,208H116a8,8,0,0,1,0-16h24a8,8,0,0,1,0,16Z"/></svg> Remote AI Control</p>
                <h1 class="hero-headline">
                    Control <span class="highlight">Claude Code</span> from anywhere
                </h1>
                <p class="hero-subheadline">
                    Answer permissions, approve plans, and respond to questions—all from your phone.
                    Claude Code runs natively on your Mac, exactly as intended.
                </p>
                <div class="hero-ctas">
                    <a href="#install" class="btn-primary">Install Now</a>
                    <a href="/dashboard" class="btn-secondary">View Demo Dashboard</a>
                </div>
            </div>

            <div class="hero-devices">
                <div class="hero-terminal">
                    <div class="terminal-window">
                    <div class="terminal-bar">
                        <div class="terminal-dot red"></div>
                        <div class="terminal-dot yellow"></div>
                        <div class="terminal-dot green"></div>
                        <span class="terminal-title">crabigator — ~/projects/api</span>
                    </div>
                    <div class="terminal-content">
                        <div class="terminal-line">
                            <span class="terminal-prompt"><svg class="icon" viewBox="0 0 256 256"><path d="M181.66,133.66l-80,80A8,8,0,0,1,88,208V48a8,8,0,0,1,13.66-5.66l80,80A8,8,0,0,1,181.66,133.66Z"/></svg></span>
                            <span class="terminal-cmd"> crabigator</span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-output">Starting Claude Code session...</span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-success"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> Connected to drinkcrabigator.com</span>
                        </div>
                        <div class="terminal-line" style="margin-top: 16px;">
                            <span class="terminal-output">╭─ Claude is thinking </span>
                            <span class="terminal-thinking"><span></span><span></span><span></span></span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-output">│ Analyzing codebase structure</span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-output">│ Reading src/app.rs</span>
                        </div>

                        <div class="terminal-widgets">
                            <div class="widget-card">
                                <div class="widget-header"><svg class="icon-widget" viewBox="0 0 256 256"><path d="M232,208a8,8,0,0,1-8,8H32a8,8,0,0,1-8-8V48a8,8,0,0,1,16,0V156.69l50.34-50.35a8,8,0,0,1,11.32,0L128,132.69,180.69,80H160a8,8,0,0,1,0-16h40a8,8,0,0,1,8,8v40a8,8,0,0,1-16,0V91.31l-58.34,58.35a8,8,0,0,1-11.32,0L96,123.31l-56,56V200H224A8,8,0,0,1,232,208Z"/></svg> Session Stats <span class="widget-state thinking">thinking</span></div>
                                <div class="widget-row">
                                    <span class="widget-label">◆ Session</span>
                                    <span class="widget-value blue">12m 34s</span>
                                </div>
                                <div class="widget-row">
                                    <span class="widget-label">▸ Prompts 7</span>
                                    <span class="widget-value dim">2m ago</span>
                                </div>
                                <div class="widget-row">
                                    <span class="widget-label">⚙ Tools</span>
                                    <span class="widget-sparkline">▁▂▄▅▇█▆▄▂▃▅▄</span>
                                </div>
                                <div class="widget-row">
                                    <span class="widget-label">⊜ Compactions 1</span>
                                    <span class="widget-value dim">5m ago</span>
                                </div>
                            </div>
                            <div class="widget-card">
                                <div class="widget-header">
                                    <svg class="icon-widget" viewBox="0 0 256 256"><path d="M216,72H131.31L104,44.69A15.86,15.86,0,0,0,92.69,40H40A16,16,0,0,0,24,56V200.62A15.4,15.4,0,0,0,39.38,216H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Z"/></svg>
                                    <span class="widget-branch">main</span>
                                    <span class="widget-files">3 files</span>
                                </div>
                                <div class="git-files">
                                    <div class="git-file">
                                        <span class="git-status modified">●</span>
                                        <span class="git-path">src/app.rs</span>
                                        <span class="git-diff"><span class="del-num">−42</span><span class="bars"><span class="bar-del">▓▓</span><span class="bar-add">███</span></span><span class="add-num">+87</span></span>
                                    </div>
                                    <div class="git-file">
                                        <span class="git-status modified">●</span>
                                        <span class="git-path">src/ui.rs</span>
                                        <span class="git-diff"><span class="del-num">−12</span><span class="bars"><span class="bar-del">▓▓</span><span class="bar-add">██</span></span><span class="add-num">+34</span></span>
                                    </div>
                                    <div class="git-file">
                                        <span class="git-status added">+</span>
                                        <span class="git-path">src/update.rs</span>
                                        <span class="git-diff"><span class="del-num"></span><span class="bars"><span class="bar-del"></span><span class="bar-add">██</span></span><span class="add-num">+21</span></span>
                                    </div>
                                </div>
                            </div>
                            <div class="widget-card changes">
                                <div class="widget-header">
                                    <svg class="icon-widget" viewBox="0 0 256 256"><path d="M200,204.5V232a8,8,0,0,1-16,0V204.5a63.67,63.67,0,0,1-35.38-18.24L124.31,162H72a8,8,0,0,1,0-16h44.69l30.35-30.34A63.65,63.65,0,0,1,184,97.5V56a8,8,0,0,1,16,0V97.5a79.58,79.58,0,0,0,22.63,55.37l5.66,5.66a8,8,0,0,1-11.32,11.32l-5.65-5.66A79.75,79.75,0,0,0,200,204.5Z"/></svg>
                                    <span class="widget-lang">TypeScript</span>
                                    <span class="widget-count">4 changes</span>
                                </div>
                                <div class="changes-list">
                                    <div class="change-item">
                                        <span class="change-mod">~</span><span class="change-icon fn">ƒ</span>
                                        <span class="change-name">handleSSE</span>
                                        <span class="change-stats"><span class="del">−8</span> <span class="add">+24</span></span>
                                    </div>
                                    <div class="change-item">
                                        <span class="change-mod">+</span><span class="change-icon fn">ƒ</span>
                                        <span class="change-name">parseEvent</span>
                                        <span class="change-stats"><span class="add">+42</span></span>
                                    </div>
                                    <div class="change-item">
                                        <span class="change-mod">~</span><span class="change-icon cls">◆</span>
                                        <span class="change-name">SessionDO</span>
                                        <span class="change-stats"><span class="del">−3</span> <span class="add">+18</span></span>
                                    </div>
                                </div>
                                <div class="changes-lang-header">
                                    <span class="widget-lang rust">Rust</span>
                                    <span class="widget-count">2 changes</span>
                                </div>
                                <div class="changes-list">
                                    <div class="change-item">
                                        <span class="change-mod">~</span><span class="change-icon fn">ƒ</span>
                                        <span class="change-name">update_widgets</span>
                                        <span class="change-stats"><span class="del">−15</span> <span class="add">+31</span></span>
                                    </div>
                                    <div class="change-item">
                                        <span class="change-mod">+</span><span class="change-icon struct">◇</span>
                                        <span class="change-name">UpdateState</span>
                                        <span class="change-stats"><span class="add">+26</span></span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                </div>
                <div class="hero-phone">
                    <div class="phone-notch"></div>
                    <div class="phone-screen">
                        <div class="phone-header">
                            <svg class="phone-logo" viewBox="0 0 512 512"><path fill="#f97316" d="M379.204,266.236c-1.879,0-3.78-0.07-5.648-0.206c-11.05-0.806-19.356-10.418-18.55-21.468c0.807-11.05,10.416-19.352,21.469-18.55c0.9,0.066,1.819,0.099,2.729,0.099c20.173,0,36.585-16.412,36.585-36.585c0-1.337-0.072-2.687-0.215-4.01c-1.193-11.016,6.77-20.913,17.786-22.106c11.014-1.19,20.913,6.77,22.105,17.786c0.298,2.751,0.449,5.553,0.449,8.329C455.914,231.824,421.502,266.236,379.204,266.236z"/><ellipse fill="#fb923c" cx="255.996" cy="294.45" rx="144.436" ry="120.976"/><path fill="#f97316" d="M400.43,294.451c0-66.813-64.664-120.975-144.431-120.976v241.952C335.767,415.428,400.43,361.265,400.43,294.451z"/></svg>
                            <span class="phone-title">~/projects/api</span>
                            <span class="phone-state thinking">thinking</span>
                        </div>
                        <div class="phone-terminal">
                            <div class="pt-line"><span class="pt-prompt">❯</span> crabigator</div>
                            <div class="pt-line pt-dim">Starting Claude Code session...</div>
                            <div class="pt-line pt-success">✓ Streaming</div>
                            <div class="pt-line pt-dim" style="margin-top: 8px;">╭─ Claude is thinking <span class="pt-thinking"><span></span><span></span><span></span></span></div>
                            <div class="pt-line pt-dim">│ Analyzing codebase structure</div>
                            <div class="pt-line pt-dim">│ Reading src/app.rs</div>
                        </div>
                        <div class="phone-widget">
                            <div class="phone-widget-header">
                                <span class="pw-icon">◆</span> Session
                                <span class="pw-value">12m 34s</span>
                            </div>
                            <div class="phone-widget-row">
                                <span>▸ Prompts 7</span>
                                <span class="pw-dim">2m ago</span>
                            </div>
                            <div class="phone-widget-row">
                                <span>⚙ Tools</span>
                                <span class="pw-sparkline">▁▂▄▃▅▇█▆▄▂▁▃▅▄</span>
                            </div>
                        </div>
                        <div class="phone-widget">
                            <div class="phone-widget-header">
                                <span class="pw-branch">main</span>
                                <span class="pw-files">3 files</span>
                            </div>
                            <div class="phone-git-file">
                                <span class="pf-status">●</span>
                                <span class="pf-path">src/app.rs</span>
                                <span class="pf-diff"><span class="del">−42</span><span class="add">+87</span></span>
                            </div>
                            <div class="phone-git-file">
                                <span class="pf-status">●</span>
                                <span class="pf-path">src/ui.rs</span>
                                <span class="pf-diff"><span class="del">−12</span><span class="add">+34</span></span>
                            </div>
                        </div>
                        <div class="phone-widget">
                            <div class="phone-widget-header">
                                <span class="pw-lang">TypeScript</span>
                                <span class="pw-count">4 changes</span>
                            </div>
                            <div class="phone-change">
                                <span class="pc-mod">~</span><span class="pc-icon">ƒ</span>
                                <span class="pc-name">handleSSE</span>
                            </div>
                            <div class="phone-change">
                                <span class="pc-mod">+</span><span class="pc-icon">ƒ</span>
                                <span class="pc-name">parseEvent</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </section>

    <!-- Stay in the Loop CTA -->
    <section class="section cta-section">
        <div class="cta-content">
            <div class="cta-card">
                <div class="cta-header">
                    <svg class="cta-icon" viewBox="0 0 20 20" fill="currentColor">
                        <path d="M2.003 5.884L10 9.882l7.997-3.998A2 2 0 0016 4H4a2 2 0 00-1.997 1.884z"/>
                        <path d="M18 8.118l-8 4-8-4V14a2 2 0 002 2h12a2 2 0 002-2V8.118z"/>
                    </svg>
                    <span class="cta-label">Stay in the loop</span>
                </div>
                <p class="cta-text">Get updates on new features and mobile app launches.</p>
                <form class="cta-form" id="hero-email-form">
                    <input type="email" class="cta-input" placeholder="you@example.com" required>
                    <button type="submit" class="cta-btn">Subscribe</button>
                </form>
                <div class="cta-success" id="hero-email-success">
                    <svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> You're on the list!
                </div>
            </div>
        </div>
    </section>

    <!-- Device Showcase -->
    <section class="section showcase">
        <div class="showcase-content">
            <div class="showcase-text">
                <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M224,72H208V48a24,24,0,0,0-24-24H40A24,24,0,0,0,16,48V152a24,24,0,0,0,24,24H152v24a24,24,0,0,0,24,24h48a24,24,0,0,0,24-24V96A24,24,0,0,0,224,72ZM40,160a8,8,0,0,1-8-8V48a8,8,0,0,1,8-8H184a8,8,0,0,1,8,8V72H176a24,24,0,0,0-24,24v64Zm192,40a8,8,0,0,1-8,8H176a8,8,0,0,1-8-8V96a8,8,0,0,1,8-8h48a8,8,0,0,1,8,8Z"/></svg> Multi-device</p>
                <h2 class="section-title">Check in from phone or desktop</h2>
                <p class="section-subtitle">
                    No Tmux. No Tailscale. No Termius. Native integration built for Claude Code.
                    Pair any device in seconds—monitor progress and respond to prompts on the go.
                </p>
                <div class="showcase-status">
                    <div class="status-pills">
                        <span class="status-pill thinking">Thinking</span>
                        <span class="status-pill permission">Permission</span>
                        <span class="status-pill question">Question</span>
                        <span class="status-pill complete">Complete</span>
                    </div>
                    <p class="status-hint">Real-time state updates across all sessions</p>
                </div>
            </div>
            <div class="showcase-devices">
                <div class="device-phone">
                    <div class="phone-screen">
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/api</span>
                                <span class="session-state thinking">thinking</span>
                            </div>
                            <div class="session-preview">
                                Reading config.ts...<br>
                                Analyzing function signatures
                            </div>
                        </div>
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/web</span>
                                <span class="session-state permission">permission</span>
                            </div>
                            <div class="session-preview">
                                Allow: npm install<br>
                                [Allow] [Deny]
                            </div>
                        </div>
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/cli</span>
                                <span class="session-state complete">complete</span>
                            </div>
                            <div class="session-preview">
                                <svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> Refactored auth module<br>
                                <svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> Added unit tests
                            </div>
                        </div>
                    </div>
                </div>
                <div class="device-browser">
                    <div class="browser-bar">
                        <div class="browser-dots">
                            <div class="browser-dot"></div>
                            <div class="browser-dot"></div>
                            <div class="browser-dot"></div>
                        </div>
                        <div class="browser-url">drinkcrabigator.com/dashboard</div>
                    </div>
                    <div class="browser-content">
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/api</span>
                                <span class="session-state thinking">thinking</span>
                            </div>
                            <div class="session-preview">
                                ╭─ Claude is thinking...<br>
                                │ Reading config.ts<br>
                                │ Analyzing function signatures
                            </div>
                        </div>
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/web</span>
                                <span class="session-state permission">permission</span>
                            </div>
                            <div class="session-preview">
                                Claude wants to: npm install lodash<br>
                                <span style="color: #4ade80;">[Allow]</span> <span style="color: #f87171;">[Deny]</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </section>

    <!-- Interactive Section -->
    <section class="section interactive">
        <div class="interactive-content">
            <div class="interactive-visual">
                <div class="prompt-mockup">
                    <div class="prompt-mockup-header">
                        <span class="prompt-mockup-state">Permission Request</span>
                        <span class="prompt-mockup-path">~/projects/api</span>
                    </div>
                    <div class="prompt-mockup-question">
                        Claude wants to run: <code>pnpm install lodash</code>
                    </div>
                    <div class="prompt-mockup-options">
                        <div class="prompt-mockup-row">
                            <div class="prompt-mockup-option selected">
                                <span class="option-num">1.</span> Yes, allow this
                            </div>
                            <input type="text" class="prompt-mockup-input" placeholder="+ instructions" value="use npm instead">
                        </div>
                        <div class="prompt-mockup-row">
                            <div class="prompt-mockup-option no-input">
                                <span class="option-num">2.</span> Yes, don't ask again
                            </div>
                        </div>
                        <div class="prompt-mockup-row">
                            <div class="prompt-mockup-option">
                                <span class="option-num">3.</span> No, deny this
                            </div>
                            <input type="text" class="prompt-mockup-input" placeholder="+ instructions">
                        </div>
                    </div>
                </div>
            </div>
            <div class="interactive-text">
                <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm36.71,118.79-48,36A8,8,0,0,1,104,172V100a8,8,0,0,1,12.71-6.47l48,36a8,8,0,0,1,0,12.94Z"/></svg> Interactive</p>
                <h2 class="section-title">Respond from anywhere</h2>
                <p class="section-subtitle">
                    Answer permissions, review plans, and respond to questions—all from your phone or any browser.
                    Add custom instructions before approving to guide Claude's next steps.
                </p>
                <ul class="interactive-features">
                    <li>
                        <span class="feature-icon"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span>
                        <span class="feature-text"><strong>Permission prompts</strong> with one-tap approve or deny</span>
                    </li>
                    <li>
                        <span class="feature-icon"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span>
                        <span class="feature-text"><strong>Add instructions</strong> before approving—guide Claude's approach</span>
                    </li>
                    <li>
                        <span class="feature-icon"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span>
                        <span class="feature-text"><strong>Answer questions</strong> when Claude needs clarification</span>
                    </li>
                    <li>
                        <span class="feature-icon"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span>
                        <span class="feature-text"><strong>Review plans</strong> before Claude starts implementing</span>
                    </li>
                </ul>
            </div>
        </div>
    </section>

    <!-- Features Bento Grid -->
    <section class="section features-section" id="features">
        <div class="section-header">
            <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M176,48H80A56,56,0,0,0,24,104v48a56,56,0,0,0,56,56h96a56,56,0,0,0,56-56V104A56,56,0,0,0,176,48ZM80,176a24,24,0,1,1,24-24A24,24,0,0,1,80,176Zm96,0a24,24,0,1,1,24-24A24,24,0,0,1,176,176Z"/></svg> Features</p>
            <h2 class="section-title">Everything you need to stay connected</h2>
        </div>
        <div class="bento-grid">
            <!-- Row 1-2: Session Stats (tall) + Git Changes (tall) + Cloud Dashboard + Mobile -->
            <div class="bento-card tall">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M232,208a8,8,0,0,1-8,8H32a8,8,0,0,1-8-8V48a8,8,0,0,1,16,0V156.69l50.34-50.35a8,8,0,0,1,11.32,0L128,132.69,180.69,80H160a8,8,0,0,1,0-16h40a8,8,0,0,1,8,8v40a8,8,0,0,1-16,0V91.31l-58.34,58.35a8,8,0,0,1-11.32,0L96,123.31l-56,56V200H224A8,8,0,0,1,232,208Z"/></svg></div>
                <h3 class="bento-title">Session Statistics</h3>
                <p class="bento-desc">Real-time metrics that update as Claude works. Track prompts, completions, tool calls, and session duration at a glance.</p>
                <div class="bento-visual">
                    <div class="mini-widget">
                        <div class="widget-header-mini"><svg class="icon-widget" viewBox="0 0 256 256"><path d="M232,208a8,8,0,0,1-8,8H32a8,8,0,0,1-8-8V48a8,8,0,0,1,16,0V156.69l50.34-50.35a8,8,0,0,1,11.32,0L128,132.69,180.69,80H160a8,8,0,0,1,0-16h40a8,8,0,0,1,8,8v40a8,8,0,0,1-16,0V91.31l-58.34,58.35a8,8,0,0,1-11.32,0L96,123.31l-56,56V200H224A8,8,0,0,1,232,208Z"/></svg> Session Stats</div>
                        <div class="widget-row-mini">
                            <span>Session</span>
                            <span class="val">34m</span>
                        </div>
                        <div class="widget-row-mini">
                            <span>Thinking</span>
                            <span class="val">16m</span>
                        </div>
                        <div class="widget-row-mini">
                            <span>Prompts</span>
                            <span class="val">17</span>
                        </div>
                        <div class="widget-row-mini">
                            <span>Completions</span>
                            <span class="val">16</span>
                        </div>
                        <div class="widget-row-mini">
                            <span>Tools</span>
                            <span class="sparkline">
                                <span class="spark" style="height: 4px;"></span>
                                <span class="spark" style="height: 8px;"></span>
                                <span class="spark" style="height: 6px;"></span>
                                <span class="spark" style="height: 12px;"></span>
                                <span class="spark" style="height: 10px;"></span>
                                <span class="spark" style="height: 14px;"></span>
                                <span class="spark" style="height: 8px;"></span>
                                <span class="spark" style="height: 6px;"></span>
                                <span class="spark" style="height: 10px;"></span>
                                <span class="spark" style="height: 12px;"></span>
                                <span class="spark" style="height: 8px;"></span>
                                <span class="spark" style="height: 4px;"></span>
                            </span>
                        </div>
                    </div>
                </div>
            </div>
            <div class="bento-card tall">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M216,72H131.31L104,44.69A15.86,15.86,0,0,0,92.69,40H40A16,16,0,0,0,24,56V200.62A15.4,15.4,0,0,0,39.38,216H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Z"/></svg></div>
                <h3 class="bento-title">Git Changes</h3>
                <p class="bento-desc">See every file Claude modifies with visual diff bars. Additions in green, deletions in red—know exactly what's changing.</p>
                <div class="bento-visual">
                    <div class="mini-widget">
                        <div class="widget-header-mini"><svg class="icon-widget" viewBox="0 0 256 256"><path d="M216,72H131.31L104,44.69A15.86,15.86,0,0,0,92.69,40H40A16,16,0,0,0,24,56V200.62A15.4,15.4,0,0,0,39.38,216H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Z"/></svg> 6 files</div>
                        <div class="file-row">
                            <span class="file-name">● css.ts</span>
                            <span class="diff-bar"><span class="del" style="width: 8px;"></span><span class="add" style="width: 45px;"></span></span>
                        </div>
                        <div class="file-row">
                            <span class="file-name">? update.rs</span>
                            <span class="diff-bar"><span class="add" style="width: 40px;"></span></span>
                        </div>
                        <div class="file-row">
                            <span class="file-name">● landing.ts</span>
                            <span class="diff-bar"><span class="del" style="width: 12px;"></span><span class="add" style="width: 30px;"></span></span>
                        </div>
                        <div class="file-row">
                            <span class="file-name">● js.ts</span>
                            <span class="diff-bar"><span class="del" style="width: 10px;"></span><span class="add" style="width: 18px;"></span></span>
                        </div>
                        <div class="file-row">
                            <span class="file-name">● main.rs</span>
                            <span class="diff-bar"><span class="del" style="width: 3px;"></span><span class="add" style="width: 9px;"></span></span>
                        </div>
                    </div>
                </div>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M160.06,40A88.1,88.1,0,0,0,81.29,88.67,64,64,0,1,0,72,216h88a88,88,0,0,0,.06-176Z"/></svg></div>
                <h3 class="bento-title">Cloud Dashboard</h3>
                <p class="bento-desc">View sessions and respond to prompts from any browser. No VPN needed.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M176,16H80A24,24,0,0,0,56,40V216a24,24,0,0,0,24,24h96a24,24,0,0,0,24-24V40A24,24,0,0,0,176,16ZM140,208H116a8,8,0,0,1,0-16h24a8,8,0,0,1,0,16Z"/></svg></div>
                <h3 class="bento-title">Mobile Access</h3>
                <p class="bento-desc">Approve permissions and answer questions from your phone.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon anthropic"><svg viewBox="0 0 256 256"><path d="M232,80.18v95.64a16,16,0,0,1-8.32,14l-80,43.41a16,16,0,0,1-15.36,0l-80-43.41a16,16,0,0,1-8.32-14V80.18a16,16,0,0,1,8.32-14l80-43.41a16,16,0,0,1,15.36,0l80,43.41A16,16,0,0,1,232,80.18Z"/></svg></div>
                <h3 class="bento-title">Claude Code</h3>
                <p class="bento-desc">First-class support with deep hook integration.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon openai"><svg viewBox="0 0 256 256"><path d="M240,128a15.85,15.85,0,0,1-4.67,11.28l-96.05,96.06a16,16,0,0,1-22.56,0l-96-96.06a16,16,0,0,1,0-22.56l96.05-96.06a16,16,0,0,1,22.56,0l96.05,96.06A15.85,15.85,0,0,1,240,128Z"/></svg></div>
                <h3 class="bento-title">Codex CLI</h3>
                <p class="bento-desc">Works seamlessly with OpenAI's Codex CLI.</p>
            </div>

            <!-- Row 3-4: Semantic Diff (wide + tall) + File Links (tall) -->
            <div class="bento-card wide tall">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M200,204.5V232a8,8,0,0,1-16,0V204.5a63.67,63.67,0,0,0-35.38-57.25l-48.4-24.19A79.58,79.58,0,0,1,56,51.5V24a8,8,0,0,1,16,0V51.5a63.67,63.67,0,0,0,35.38,57.25l48.4,24.19A79.58,79.58,0,0,1,200,204.5ZM164.62,64.75,142.47,53.68A79.58,79.58,0,0,0,98.22,35.94,63.67,63.67,0,0,1,72,51.5V72a8,8,0,0,1-16,0V51.5a79.58,79.58,0,0,0-32.78-64.34A8,8,0,0,0,8,0V51.5A79.58,79.58,0,0,0,52.25,123.06l22.13,11.07A79.58,79.58,0,0,1,118.6,151.87,63.67,63.67,0,0,0,144,136.31V116a8,8,0,0,1,16,0v20.31a79.58,79.58,0,0,1,32.78,64.34A8,8,0,0,0,208,188V136.31A79.58,79.58,0,0,0,164.62,64.75Z"/></svg></div>
                <h3 class="bento-title">Semantic Diff Parsing</h3>
                <p class="bento-desc">Changes grouped by language with function and method names extracted. See exactly which functions are being modified, not just file names.</p>
                <div class="bento-visual">
                    <div class="mini-widget semantic">
                        <div class="lang-group">
                            <div class="lang-header"><span class="lang-tag js">JavaScript</span> 5 changes</div>
                            <div class="func-row"><span class="func-icon">~ƒ</span> handleEmailSignup <span class="func-diff"><span class="red">-20</span> <span class="green">+33</span></span></div>
                            <div class="func-row"><span class="func-icon">~ƒ</span> copyInstallCommand <span class="func-diff"><span class="red">-8</span> <span class="green">+12</span></span></div>
                            <div class="func-row"><span class="func-icon">+ƒ</span> initDashboard <span class="func-diff"><span class="green">+45</span></span></div>
                        </div>
                        <div class="lang-group">
                            <div class="lang-header"><span class="lang-tag rs">Rust</span> 4 changes</div>
                            <div class="func-row"><span class="func-icon">~ƒ</span> main <span class="func-diff"><span class="red">-2</span> <span class="green">+5</span></span></div>
                            <div class="func-row"><span class="func-icon">+ƒ</span> default_true <span class="func-diff"><span class="green">+2</span></span></div>
                            <div class="func-row"><span class="func-icon">~◇</span> Default for Config <span class="func-diff"><span class="red">-1</span> <span class="green">+3</span></span></div>
                        </div>
                    </div>
                </div>
            </div>
            <div class="bento-card tall">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M208,32H48A16,16,0,0,0,32,48V208a16,16,0,0,0,16,16H208a16,16,0,0,0,16-16V48A16,16,0,0,0,208,32ZM144,176H112a40,40,0,0,1,0-80h32a8,8,0,0,1,0,16H112a24,24,0,0,0,0,48h32a8,8,0,0,1,0,16Zm48-56H168a8,8,0,0,1,0-16h24a24,24,0,0,0,0-48H168a8,8,0,0,1,0-16h24a40,40,0,0,1,0,80Z"/></svg></div>
                <h3 class="bento-title">Clickable File Links</h3>
                <p class="bento-desc">Every file path is a link. Click to open directly in VS Code, Cursor, Zed, or your preferred editor.</p>
                <div class="bento-visual">
                    <div class="mini-widget file-links">
                        <div class="link-row"><span class="link-path">src/app.rs</span><span class="link-line">:142</span></div>
                        <div class="link-row"><span class="link-path">src/config.ts</span><span class="link-line">:38</span></div>
                        <div class="link-row"><span class="link-path">tests/fixture.rs</span><span class="link-line">:256</span></div>
                        <div class="link-row"><span class="link-path">src/main.rs</span><span class="link-line">:89</span></div>
                    </div>
                </div>
            </div>

            <!-- Row 5: CLI + Scrollback + Mouse + ? -->
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M168,112a56,56,0,1,1-56-56A56.06,56.06,0,0,1,168,112Zm61.66,117.66a8,8,0,0,1-11.32,0l-50.06-50.07a88,88,0,1,1,11.32-11.31l50.06,50.06A8,8,0,0,1,229.66,229.66Z"/></svg></div>
                <h3 class="bento-title">CLI Inspection</h3>
                <p class="bento-desc">Use <code>crabigator inspect</code> to view running instances. Perfect for automation.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M224,128a8,8,0,0,1-8,8H40a8,8,0,0,1,0-16H216A8,8,0,0,1,224,128ZM40,72H216a8,8,0,0,0,0-16H40a8,8,0,0,0,0,16Zm104,112H40a8,8,0,0,0,0,16H144a8,8,0,0,0,0-16Z"/></svg></div>
                <h3 class="bento-title">Native Scrollback</h3>
                <p class="bento-desc">Uses your terminal's primary buffer—unlike tmux. Scroll naturally.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M144,16H112A64.07,64.07,0,0,0,48,80v96a64.07,64.07,0,0,0,64,64h32a64.07,64.07,0,0,0,64-64V80A64.07,64.07,0,0,0,144,16Zm-8,64a8,8,0,0,1-16,0V48a8,8,0,0,1,16,0Z"/></svg></div>
                <h3 class="bento-title">Mouse Selection</h3>
                <p class="bento-desc">Select and copy text naturally—no tmux capture mode.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M213.85,125.46l-112,120a8,8,0,0,1-13.69-7l14.66-73.33L45.19,143.49a8,8,0,0,1-3-13l112-120a8,8,0,0,1,13.69,7L153.18,90.9l57.63,21.61a8,8,0,0,1,3,12.95Z"/></svg></div>
                <h3 class="bento-title">Instant Pairing</h3>
                <p class="bento-desc">Scan a QR code or enter a short code. Connected in seconds.</p>
            </div>
        </div>
    </section>

    <!-- Pricing Section -->
    <section class="section pricing" id="pricing">
        <div class="section-header">
            <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm12,152h-4v8a8,8,0,0,1-16,0v-8H104a8,8,0,0,1,0-16h36a12,12,0,0,0,0-24H116a28,28,0,0,1,0-56h4V72a8,8,0,0,1,16,0v8h16a8,8,0,0,1,0,16H116a12,12,0,0,0,0,24h24a28,28,0,0,1,0,56Z"/></svg> Pricing</p>
            <h2 class="section-title">Simple, transparent pricing</h2>
        </div>
        <div class="pricing-cards">
            <div class="pricing-card">
                <div class="pricing-inner">
                    <div class="pricing-tier">Free</div>
                    <div class="pricing-amount">$0<span class="pricing-period">/month</span></div>
                    <ul class="pricing-features">
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited Claude Code sessions</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Answer permissions & questions</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited web and mobile access</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Real-time sync</li>
                        <li><span class="check dim"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> <span class="dim">30 min/day remote access</span></li>
                    </ul>
                    <a href="#install" class="btn-primary pricing-cta outline">Get Started</a>
                </div>
            </div>
            <div class="pricing-card featured">
                <div class="pricing-inner">
                    <div class="pricing-tier">Pro</div>
                    <div class="pricing-amount">$3<span class="pricing-period">/month</span></div>
                    <ul class="pricing-features">
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited Claude Code sessions</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Answer permissions & questions</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited web and mobile access</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Real-time sync</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited remote access</li>
                    </ul>
                    <a href="#install" class="btn-primary pricing-cta">Get Started</a>
                </div>
            </div>
        </div>
    </section>

    <!-- Installation Section -->
    <section class="section install" id="install">
        <div class="section-header">
            <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm49.53,85.8-58.67,56a8,8,0,0,1-11.05,0l-29.33-28a8,8,0,0,1,11.05-11.62L116,149.91l53.14-50.69a8,8,0,0,1,11.06.06A8,8,0,0,1,177.53,109.8Z"/></svg> Get Started</p>
            <h2 class="section-title">Install in 30 seconds</h2>
        </div>
        <div class="install-content">
            <div class="install-terminal">
                <div class="install-terminal-bar">
                    <span class="install-terminal-label">Terminal</span>
                    <button class="copy-btn" id="copy-btn" onclick="copyInstallCommand()">
                        <svg viewBox="0 0 16 16" fill="currentColor">
                            <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/>
                            <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/>
                        </svg>
                        Copy
                    </button>
                </div>
                <div class="install-terminal-content">
                    <span class="prompt">$</span> npm install -g crabigator
                </div>
            </div>
            <div class="install-steps">
                <div class="install-step">
                    <div class="install-step-num">1</div>
                    <p class="install-step-text">Install via npm</p>
                </div>
                <div class="install-step">
                    <div class="install-step-num">2</div>
                    <p class="install-step-text">Run <code>crabigator</code> instead of <code>claude</code></p>
                </div>
                <div class="install-step">
                    <div class="install-step-num">3</div>
                    <p class="install-step-text">Click the pairing link to connect your phone</p>
                </div>
            </div>
        </div>
    </section>

    <!-- Open Source Section -->
    <section class="section open-source" id="open-source">
        <div class="github-card">
            <div class="github-header">
                <svg class="github-logo" viewBox="0 0 98 96" fill="currentColor">
                    <path fill-rule="evenodd" clip-rule="evenodd" d="M48.854 0C21.839 0 0 22 0 49.217c0 21.756 13.993 40.172 33.405 46.69 2.427.49 3.316-1.059 3.316-2.362 0-1.141-.08-5.052-.08-9.127-13.59 2.934-16.42-5.867-16.42-5.867-2.184-5.704-5.42-7.17-5.42-7.17-4.448-3.015.324-3.015.324-3.015 4.934.326 7.523 5.052 7.523 5.052 4.367 7.496 11.404 5.378 14.235 4.074.404-3.178 1.699-5.378 3.074-6.6-10.839-1.141-22.243-5.378-22.243-24.283 0-5.378 1.94-9.778 5.014-13.2-.485-1.222-2.184-6.275.486-13.038 0 0 4.125-1.304 13.426 5.052a46.97 46.97 0 0 1 12.214-1.63c4.125 0 8.33.571 12.213 1.63 9.302-6.356 13.427-5.052 13.427-5.052 2.67 6.763.97 11.816.485 13.038 3.155 3.422 5.015 7.822 5.015 13.2 0 18.905-11.404 23.06-22.324 24.283 1.78 1.548 3.316 4.481 3.316 9.126 0 6.6-.08 11.897-.08 13.526 0 1.304.89 2.853 3.316 2.364 19.412-6.52 33.405-24.935 33.405-46.691C97.707 22 75.788 0 48.854 0z"/>
                </svg>
                <div class="github-repo">
                    <span class="github-org">samuelclay</span>
                    <span class="github-sep">/</span>
                    <span class="github-name">crabigator</span>
                </div>
            </div>
            <p class="github-desc">
                A Rust TUI wrapper for Claude Code with real-time mobile streaming. MIT licensed.
            </p>
            <div class="github-meta">
                <span class="github-lang">
                    <span class="lang-dot rust"></span>
                    Rust
                </span>
                <span class="github-license">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M8.75.75V2h.985c.304 0 .603.08.867.231l1.29.736c.038.022.08.033.124.033h2.234a.75.75 0 0 1 0 1.5h-.427l2.111 4.692a.75.75 0 0 1-.154.838l-.53-.53.529.531-.001.002-.002.002-.006.006-.016.015-.045.04a3.514 3.514 0 0 1-.686.45A4.492 4.492 0 0 1 13 11c-.88 0-1.556-.22-2.023-.454a3.515 3.515 0 0 1-.686-.45l-.045-.04-.016-.015-.006-.006-.004-.004-.001-.001a.75.75 0 0 1-.154-.838L12.178 4.5h-.162c-.305 0-.604-.079-.868-.231l-1.29-.736a.245.245 0 0 0-.124-.033H8.75V13h2.5a.75.75 0 0 1 0 1.5h-6.5a.75.75 0 0 1 0-1.5h2.5V3.5h-.984a.245.245 0 0 0-.124.033l-1.289.737c-.265.15-.564.23-.869.23h-.162l2.112 4.692a.75.75 0 0 1-.154.838l-.53-.53.529.531-.001.002-.002.002-.006.006-.016.015-.045.04a3.517 3.517 0 0 1-.686.45A4.492 4.492 0 0 1 3 11c-.88 0-1.556-.22-2.023-.454a3.512 3.512 0 0 1-.686-.45l-.045-.04-.016-.015-.006-.006-.004-.004-.001-.001a.75.75 0 0 1-.154-.838L2.178 4.5H1.75a.75.75 0 0 1 0-1.5h2.234c.044 0 .086-.011.124-.033l1.29-.736A1.75 1.75 0 0 1 6.265 2H7.25V.75a.75.75 0 0 1 1.5 0Z"/></svg>
                    MIT
                </span>
            </div>
            <div class="github-actions">
                <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="github-btn primary">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M8 .25a.75.75 0 0 1 .673.418l1.882 3.815 4.21.612a.75.75 0 0 1 .416 1.279l-3.046 2.97.719 4.192a.751.751 0 0 1-1.088.791L8 12.347l-3.766 1.98a.75.75 0 0 1-1.088-.79l.72-4.194L.818 6.374a.75.75 0 0 1 .416-1.28l4.21-.611L7.327.668A.75.75 0 0 1 8 .25Z"/></svg>
                    Star
                </a>
                <a href="https://github.com/samuelclay/crabigator/fork" target="_blank" rel="noopener" class="github-btn">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M5 5.372v.878c0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75v-.878a2.25 2.25 0 1 1 1.5 0v.878a2.25 2.25 0 0 1-2.25 2.25h-1.5v2.128a2.251 2.251 0 1 1-1.5 0V8.5h-1.5A2.25 2.25 0 0 1 3.5 6.25v-.878a2.25 2.25 0 1 1 1.5 0ZM5 3.25a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Zm6.75.75a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Zm-3 8.75a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Z"/></svg>
                    Fork
                </a>
                <a href="https://github.com/samuelclay/crabigator/issues" target="_blank" rel="noopener" class="github-btn">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M8 9.5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Z"/><path d="M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0ZM1.5 8a6.5 6.5 0 1 0 13 0 6.5 6.5 0 0 0-13 0Z"/></svg>
                    Issues
                </a>
                <a href="https://github.com/samuelclay/crabigator/pulls" target="_blank" rel="noopener" class="github-btn">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M1.5 3.25a2.25 2.25 0 1 1 3 2.122v5.256a2.251 2.251 0 1 1-1.5 0V5.372A2.25 2.25 0 0 1 1.5 3.25Zm5.677-.177L9.573.677A.25.25 0 0 1 10 .854V2.5h1A2.5 2.5 0 0 1 13.5 5v5.628a2.251 2.251 0 1 1-1.5 0V5a1 1 0 0 0-1-1h-1v1.646a.25.25 0 0 1-.427.177L7.177 3.427a.25.25 0 0 1 0-.354ZM3.75 2.5a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm0 9.5a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm8.25.75a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Z"/></svg>
                    PRs
                </a>
            </div>
            <div class="github-links">
                <a href="https://github.com/samuelclay/crabigator#readme" target="_blank" rel="noopener" class="github-link">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M0 1.75A.75.75 0 0 1 .75 1h4.253c1.227 0 2.317.59 3 1.501A3.743 3.743 0 0 1 11.006 1h4.245a.75.75 0 0 1 .75.75v10.5a.75.75 0 0 1-.75.75h-4.507a2.25 2.25 0 0 0-1.591.659l-.622.621a.75.75 0 0 1-1.06 0l-.622-.621A2.25 2.25 0 0 0 5.258 13H.75a.75.75 0 0 1-.75-.75Zm7.251 10.324.004-5.073-.002-2.253A2.25 2.25 0 0 0 5.003 2.5H1.5v9h3.757a3.75 3.75 0 0 1 1.994.574ZM8.755 4.75l-.004 7.322a3.752 3.752 0 0 1 1.992-.572H14.5v-9h-3.495a2.25 2.25 0 0 0-2.25 2.25Z"/></svg>
                    README
                </a>
                <a href="https://github.com/samuelclay/crabigator/blob/main/LICENSE" target="_blank" rel="noopener" class="github-link">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M8.75.75V2h.985c.304 0 .603.08.867.231l1.29.736c.038.022.08.033.124.033h2.234a.75.75 0 0 1 0 1.5h-.427l2.111 4.692a.75.75 0 0 1-.154.838l-.53-.53.529.531-.001.002-.002.002-.006.006-.016.015-.045.04a3.514 3.514 0 0 1-.686.45A4.492 4.492 0 0 1 13 11c-.88 0-1.556-.22-2.023-.454a3.515 3.515 0 0 1-.686-.45l-.045-.04-.016-.015-.006-.006-.004-.004-.001-.001a.75.75 0 0 1-.154-.838L12.178 4.5h-.162c-.305 0-.604-.079-.868-.231l-1.29-.736a.245.245 0 0 0-.124-.033H8.75V13h2.5a.75.75 0 0 1 0 1.5h-6.5a.75.75 0 0 1 0-1.5h2.5V3.5h-.984a.245.245 0 0 0-.124.033l-1.289.737c-.265.15-.564.23-.869.23h-.162l2.112 4.692a.75.75 0 0 1-.154.838l-.53-.53.529.531-.001.002-.002.002-.006.006-.016.015-.045.04a3.517 3.517 0 0 1-.686.45A4.492 4.492 0 0 1 3 11c-.88 0-1.556-.22-2.023-.454a3.512 3.512 0 0 1-.686-.45l-.045-.04-.016-.015-.006-.006-.004-.004-.001-.001a.75.75 0 0 1-.154-.838L2.178 4.5H1.75a.75.75 0 0 1 0-1.5h2.234c.044 0 .086-.011.124-.033l1.29-.736A1.75 1.75 0 0 1 6.265 2H7.25V.75a.75.75 0 0 1 1.5 0Z"/></svg>
                    LICENSE
                </a>
                <a href="https://github.com/samuelclay/crabigator/releases" target="_blank" rel="noopener" class="github-link">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M1 7.775V2.75C1 1.784 1.784 1 2.75 1h5.025c.464 0 .91.184 1.238.513l6.25 6.25a1.75 1.75 0 0 1 0 2.474l-5.026 5.026a1.75 1.75 0 0 1-2.474 0l-6.25-6.25A1.752 1.752 0 0 1 1 7.775Zm1.5 0c0 .066.026.13.073.177l6.25 6.25a.25.25 0 0 0 .354 0l5.025-5.025a.25.25 0 0 0 0-.354l-6.25-6.25a.25.25 0 0 0-.177-.073H2.75a.25.25 0 0 0-.25.25ZM6 5a1 1 0 1 1 0 2 1 1 0 0 1 0-2Z"/></svg>
                    Releases
                </a>
                <a href="https://github.com/samuelclay/crabigator/wiki" target="_blank" rel="noopener" class="github-link">
                    <svg viewBox="0 0 16 16" fill="currentColor"><path d="M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm10.5 6.5a.75.75 0 0 0 0-1.5h-8.5a.75.75 0 0 0 0 1.5Zm0-3a.75.75 0 0 0 0-1.5h-8.5a.75.75 0 0 0 0 1.5Z"/></svg>
                    Discussions
                </a>
            </div>
        </div>
    </section>

    <!-- Mobile Apps Coming Soon -->
    <section class="section mobile-apps" id="mobile">
        <div class="mobile-content">
            <div class="mobile-phones">
                <!-- iPhone -->
                <div class="phone iphone">
                    <div class="phone-notch"></div>
                    <div class="phone-screen">
                        <div class="app-header">
                            <span class="app-logo">🦀</span>
                            <span class="app-title">Crabigator</span>
                        </div>
                        <div class="app-notification">
                            <div class="notif-icon">
                                <svg viewBox="0 0 256 256" fill="currentColor"><path d="M221.8,175.94C216.25,166.38,208,139.33,208,104a80,80,0,1,0-160,0c0,35.34-8.26,62.38-13.81,71.94A16,16,0,0,0,48,200H88.81a40,40,0,0,0,78.38,0H208a16,16,0,0,0,13.8-24.06Z"/></svg>
                            </div>
                            <div class="notif-content">
                                <div class="notif-title">Permission Required</div>
                                <div class="notif-body">Claude wants to run: git commit</div>
                            </div>
                        </div>
                        <div class="app-session">
                            <div class="session-state thinking">Thinking...</div>
                            <div class="session-path">~/projects/app</div>
                        </div>
                        <div class="app-actions">
                            <button class="app-btn approve">Approve</button>
                            <button class="app-btn deny">Deny</button>
                        </div>
                    </div>
                    <div class="phone-label">
                        <svg viewBox="0 0 256 256" fill="currentColor"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.1,88.1,0,0,1,128,216ZM173.66,90.34a8,8,0,0,1,0,11.32l-40,40a8,8,0,0,1-11.32-11.32l40-40A8,8,0,0,1,173.66,90.34ZM96,16a8,8,0,0,1,8-8h48a8,8,0,0,1,0,16H104A8,8,0,0,1,96,16Z"/></svg>
                        iOS
                    </div>
                </div>
                <!-- Android -->
                <div class="phone android">
                    <div class="phone-screen">
                        <div class="app-header">
                            <span class="app-logo">🦀</span>
                            <span class="app-title">Crabigator</span>
                        </div>
                        <div class="app-terminal">
                            <div class="term-line"><span class="term-prompt">❯</span> Analyzing codebase...</div>
                            <div class="term-line"><span class="term-file">src/main.rs</span> modified</div>
                            <div class="term-line"><span class="term-added">+42</span> <span class="term-removed">-12</span></div>
                        </div>
                        <div class="app-stats">
                            <div class="stat"><span class="stat-value">3</span><span class="stat-label">Sessions</span></div>
                            <div class="stat"><span class="stat-value">47</span><span class="stat-label">Prompts</span></div>
                            <div class="stat"><span class="stat-value">2.1h</span><span class="stat-label">Time</span></div>
                        </div>
                    </div>
                    <div class="phone-label">
                        <svg viewBox="0 0 256 256" fill="currentColor"><path d="M176,16H80A24,24,0,0,0,56,40V216a24,24,0,0,0,24,24h96a24,24,0,0,0,24-24V40A24,24,0,0,0,176,16ZM72,64H184V192H72Zm8-32h96a8,8,0,0,1,8,8v8H72V40A8,8,0,0,1,80,32Zm96,192H80a8,8,0,0,1-8-8v-8H184v8A8,8,0,0,1,176,224Z"/></svg>
                        Android
                    </div>
                </div>
            </div>
            <div class="mobile-info">
                <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M176,16H80A24,24,0,0,0,56,40V216a24,24,0,0,0,24,24h96a24,24,0,0,0,24-24V40A24,24,0,0,0,176,16ZM140,208H116a8,8,0,0,1,0-16h24a8,8,0,0,1,0,16Z"/></svg> Coming Soon</p>
                <h2 class="section-title">Native mobile apps</h2>
                <div class="mobile-features">
                    <div class="mobile-feature">
                        <svg viewBox="0 0 256 256" fill="currentColor"><path d="M221.8,175.94C216.25,166.38,208,139.33,208,104a80,80,0,1,0-160,0c0,35.34-8.26,62.38-13.81,71.94A16,16,0,0,0,48,200H88.81a40,40,0,0,0,78.38,0H208a16,16,0,0,0,13.8-24.06Z"/></svg>
                        <div>
                            <strong>Push notifications</strong>
                            <span>Know instantly when Claude needs your approval</span>
                        </div>
                    </div>
                    <div class="mobile-feature">
                        <svg viewBox="0 0 256 256" fill="currentColor"><path d="M197.66,133.66l-56,56a8,8,0,0,1-11.32-11.32L172.69,136H48a8,8,0,0,1,0-16H172.69L130.34,77.66a8,8,0,0,1,11.32-11.32l56,56A8,8,0,0,1,197.66,133.66Z"/></svg>
                        <div>
                            <strong>Native performance</strong>
                            <span>Smooth 60fps animations and instant response</span>
                        </div>
                    </div>
                    <div class="mobile-feature">
                        <svg viewBox="0 0 256 256" fill="currentColor"><path d="M160,40A88.09,88.09,0,0,0,81.29,88.67,64,64,0,1,0,72,216h88a88,88,0,0,0,0-176Z"/></svg>
                        <div>
                            <strong>Offline support</strong>
                            <span>Review sessions even without internet</span>
                        </div>
                    </div>
                </div>
                <form class="email-form" id="email-form">
                    <input type="email" class="email-input" placeholder="you@example.com" required>
                    <button type="submit" class="email-btn">Notify Me</button>
                </form>
                <div class="email-success" id="email-success">
                    <svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> You're on the list!
                </div>
                <p class="email-privacy">Get notified about product launches and updates.</p>
            </div>
        </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
        <div class="footer-content">
            <div class="footer-cta">
                <span class="footer-cta-text">Ready to control Claude Code from anywhere?</span>
                <a href="#install" class="btn-primary">Install Now</a>
            </div>
            <div class="footer-links">
                <a href="/dashboard" class="footer-link">Dashboard</a>
                <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="footer-link">GitHub</a>
                <a href="https://github.com/samuelclay/crabigator#readme" target="_blank" rel="noopener" class="footer-link">Documentation</a>
            </div>
            <div class="footer-meta">
                MIT License
            </div>
        </div>
        <div class="footer-bottom">
            <p class="footer-bottom-text">Built by <a href="https://samuelclay.com" target="_blank" rel="noopener">Samuel Clay</a>. Talk to <a href="https://x.com/samuelclay" target="_blank" rel="noopener">Samuel Clay</a> on X.</p>
        </div>
    </footer>

    <script>${landingJs}</script>
</body>
</html>`;
