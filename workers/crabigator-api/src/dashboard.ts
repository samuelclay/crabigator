// Dashboard HTML served at /dashboard
import { dashboardCss } from './dashboard/css';
import { dashboardJs } from './dashboard/js';

export const dashboardHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>Crabigator Dashboard</title>
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 512 512'><path fill='%23ea580c' d='M379.204,266.236c-1.879,0-3.78-0.07-5.648-0.206c-11.05-0.806-19.356-10.418-18.55-21.468c0.807-11.05,10.416-19.352,21.469-18.55c0.9,0.066,1.819,0.099,2.729,0.099c20.173,0,36.585-16.412,36.585-36.585c0-1.337-0.072-2.687-0.215-4.01c-1.193-11.016,6.77-20.913,17.786-22.106c11.014-1.19,20.913,6.77,22.105,17.786c0.298,2.751,0.449,5.553,0.449,8.329C455.914,231.824,421.502,266.236,379.204,266.236z'/><path fill='%23f97316' d='M415.008,97.65v-53.37c4.962-1.106,10.113-1.709,15.408-1.709c38.929,0,70.489,31.558,70.489,70.487s-31.558,70.487-70.489,70.487c-38.929,0-70.487-31.558-70.487-70.487c0-5.295,0.603-10.446,1.709-15.408L415.008,97.65L415.008,97.65z'/><path fill='%23ea580c' d='M491.941,331.928c-5.383,0-10.752-2.153-14.704-6.409c-20.311-21.868-49.921-36.124-83.371-40.14c-11.001-1.321-18.849-11.31-17.528-22.311c1.321-11.001,11.302-18.851,22.311-17.528c42.871,5.148,81.222,23.853,107.989,52.672c7.541,8.119,7.073,20.813-1.046,28.354C501.726,330.153,496.828,331.928,491.941,331.928z'/><path fill='%23ea580c' d='M482.529,411.418c-6.567,0-13-3.219-16.844-9.136c-16.534-25.457-43.78-44.737-76.72-54.289c-10.642-3.086-16.767-14.215-13.68-24.856c3.086-10.642,14.212-16.766,24.856-13.681c42.186,12.234,77.414,37.438,99.195,70.969c6.035,9.292,3.396,21.718-5.897,27.753C490.063,410.369,486.274,411.418,482.529,411.418z'/><path fill='%23ea580c' d='M434.652,469.431c-7.463,0-14.63-4.182-18.087-11.357c-13.569-28.15-40.168-51.891-72.98-65.132c-10.275-4.146-15.242-15.837-11.096-26.112c4.148-10.276,15.839-15.243,26.113-11.096c42.575,17.181,75.996,47.339,94.109,84.92c4.811,9.982,0.619,21.972-9.362,26.783C440.54,468.79,437.573,469.431,434.652,469.431z'/><path fill='%23f97316' d='M132.796,266.236c-42.298,0-76.709-34.412-76.709-76.709c0-2.775,0.151-5.577,0.449-8.329c1.193-11.016,11.097-18.972,22.105-17.786c11.016,1.193,18.979,11.091,17.786,22.106c-0.143,1.323-0.215,2.672-0.215,4.01c0,20.172,16.412,36.585,36.586,36.585c0.91,0,1.828-0.033,2.729-0.099c11.062-0.804,20.663,7.499,21.469,18.55c0.805,11.05-7.499,20.663-18.55,21.468C136.575,266.167,134.675,266.236,132.796,266.236z'/><path fill='%23f97316' d='M20.058,331.928c-4.887,0-9.785-1.775-13.649-5.363c-8.119-7.541-8.587-20.235-1.046-28.354c26.769-28.819,65.12-47.524,107.989-52.672c11.009-1.317,20.989,6.527,22.311,17.528c1.321,11.001-6.527,20.991-17.528,22.311c-33.451,4.017-63.06,18.272-83.373,40.141C30.81,329.775,25.44,331.928,20.058,331.928z'/><path fill='%23f97316' d='M29.47,411.418c-3.746,0-7.534-1.047-10.909-3.239c-9.293-6.035-11.932-18.461-5.897-27.753c21.78-33.531,57.008-58.736,99.195-70.969c10.644-3.087,21.77,3.039,24.856,13.681c3.087,10.641-3.039,21.77-13.681,24.856c-32.938,9.552-60.186,28.832-76.72,54.289C42.472,408.197,36.036,411.418,29.47,411.418z'/><path fill='%23f97316' d='M77.347,469.431c-2.922,0-5.888-0.641-8.696-1.994c-9.982-4.811-14.173-16.803-9.362-26.783c18.112-37.58,51.534-67.739,94.109-84.92c10.277-4.146,21.967,0.821,26.113,11.096c4.146,10.275-0.821,21.966-11.096,26.112c-32.813,13.241-59.412,36.982-72.98,65.132C91.978,465.247,84.81,469.431,77.347,469.431z'/><ellipse fill='%23fb923c' cx='255.996' cy='294.45' rx='144.436' ry='120.976'/><path fill='%23fb923c' d='M96.991,97.65v-53.37c-4.962-1.106-10.113-1.709-15.408-1.709c-38.929,0-70.487,31.558-70.487,70.487s31.558,70.487,70.487,70.487s70.487-31.558,70.487-70.487c0-5.295-0.602-10.446-1.709-15.408L96.991,97.65L96.991,97.65z'/><path fill='%23f97316' d='M400.43,294.451c0-66.813-64.664-120.975-144.431-120.976v241.952C335.767,415.428,400.43,361.265,400.43,294.451z'/></svg>">
    <style>${dashboardCss}</style>
</head>
<body>
    <div class="deploy-overlay" id="deploy-overlay">
        <div class="deploy-spinner"></div>
        <div class="deploy-text">Reconnecting to Crabigator...</div>
        <div class="deploy-subtext">A new version was deployed</div>
        <div class="deploy-countdown" id="deploy-countdown"></div>
    </div>
    <div class="header">
        <h1>🦀 Crabigator Dashboard</h1>
        <button class="refresh-btn" onclick="loadSessions()">↻ Refresh</button>
        <div class="status" id="status">Loading...</div>
        <div class="style-container">
            <button class="style-btn" id="style-btn" onclick="toggleStylePopover()">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="3"/>
                    <path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83"/>
                </svg>
                Style
            </button>
            <div class="style-popover" id="style-popover">
                <div class="style-section">
                    <div class="style-section-label">Columns</div>
                    <div class="style-options">
                        <button class="style-option active" data-layout="1" onclick="setLayout('1')">1</button>
                        <button class="style-option" data-layout="2" onclick="setLayout('2')">2</button>
                        <button class="style-option" data-layout="3" onclick="setLayout('3')">3</button>
                        <button class="style-option" data-layout="fit" onclick="setLayout('fit')">Fit</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Text Size</div>
                    <div class="font-size-control">
                        <button class="font-size-btn decrease" onclick="adjustFontSize(-1)" title="Smaller">A</button>
                        <div class="font-size-value" id="font-label">100%</div>
                        <button class="font-size-btn increase" onclick="adjustFontSize(1)" title="Larger">A</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Terminal Height</div>
                    <div class="font-size-control">
                        <button class="font-size-btn decrease" onclick="adjustTerminalHeight(-1)" title="Shorter">−</button>
                        <div class="font-size-value" id="height-label">350px</div>
                        <button class="font-size-btn increase" onclick="adjustTerminalHeight(1)" title="Taller">+</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Text Wrap</div>
                    <div class="style-options">
                        <button class="style-option active" data-wrap="wrap" onclick="setTerminalWrap(true)">Wrap</button>
                        <button class="style-option" data-wrap="scroll" onclick="setTerminalWrap(false)">Scroll</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Widgets</div>
                    <div class="style-options">
                        <button class="style-option active" data-widgets="expanded" onclick="setWidgetsExpanded(true)">Expanded</button>
                        <button class="style-option" data-widgets="collapsed" onclick="setWidgetsExpanded(false)">Collapsed</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Grouping</div>
                    <div class="style-options">
                        <button class="style-option active" data-grouping="all" onclick="setGrouping('all')">All</button>
                        <button class="style-option" data-grouping="project" onclick="setGrouping('project')">By Project</button>
                    </div>
                </div>
            </div>
        </div>
        <div class="settings-container">
            <button class="settings-btn" id="settings-btn" onclick="toggleSettingsPopover()">
                <svg viewBox="0 0 16 16" fill="currentColor">
                    <path fill-rule="evenodd" d="M7.429 1.525a6.593 6.593 0 011.142 0c.036.003.108.036.137.146l.289 1.105c.147.56.55.967.997 1.189.174.086.341.183.501.29.417.278.97.423 1.53.27l1.102-.303c.11-.03.175.016.195.046.219.31.41.641.573.989.014.031.022.11-.059.19l-.815.806c-.411.406-.562.957-.53 1.456a4.588 4.588 0 010 .582c-.032.499.119 1.05.53 1.456l.815.806c.08.08.073.159.059.19a6.494 6.494 0 01-.573.99c-.02.029-.086.074-.195.045l-1.103-.303c-.559-.153-1.112-.008-1.529.27-.16.107-.327.204-.5.29-.449.222-.851.628-.998 1.189l-.289 1.105c-.029.11-.101.143-.137.146a6.613 6.613 0 01-1.142 0c-.036-.003-.108-.037-.137-.146l-.289-1.105c-.147-.56-.55-.967-.997-1.189a4.502 4.502 0 01-.501-.29c-.417-.278-.97-.423-1.53-.27l-1.102.303c-.11.03-.175-.016-.195-.046a6.492 6.492 0 01-.573-.989c-.014-.031-.022-.11.059-.19l.815-.806c.411-.406.562-.957.53-1.456a4.587 4.587 0 010-.582c.032-.499-.119-1.05-.53-1.456l-.815-.806c-.08-.08-.073-.159-.059-.19a6.44 6.44 0 01.573-.99c.02-.029.086-.074.195-.045l1.103.303c.559.153 1.112.008 1.529-.27.16-.107.327-.204.5-.29.449-.222.851-.628.998-1.189l.289-1.105c.029-.11.101-.143.137-.146zM8 0c-.236 0-.47.01-.701.03-.743.065-1.29.615-1.458 1.261l-.29 1.106c-.017.066-.078.158-.211.224a5.994 5.994 0 00-.668.386c-.123.082-.233.09-.3.071L3.27 2.776c-.644-.177-1.392.02-1.82.63a7.977 7.977 0 00-.704 1.217c-.315.675-.111 1.422.363 1.891l.815.806c.05.048.098.147.088.294a6.084 6.084 0 000 .772c.01.147-.038.246-.088.294l-.815.806c-.474.469-.678 1.216-.363 1.891.2.428.436.835.704 1.218.428.609 1.176.806 1.82.63l1.103-.303c.066-.019.176-.011.299.071.213.143.436.272.668.386.133.066.194.158.212.224l.289 1.106c.169.646.715 1.196 1.458 1.26a8.094 8.094 0 001.402 0c.743-.064 1.29-.614 1.458-1.26l.29-1.106c.017-.066.078-.158.211-.224a5.98 5.98 0 00.668-.386c.123-.082.233-.09.3-.071l1.102.302c.644.177 1.392-.02 1.82-.63.268-.382.505-.789.704-1.217.315-.675.111-1.422-.364-1.891l-.814-.806c-.05-.048-.098-.147-.088-.294a6.1 6.1 0 000-.772c-.01-.147.039-.246.088-.294l.814-.806c.475-.469.679-1.216.364-1.891a7.992 7.992 0 00-.704-1.218c-.428-.609-1.176-.806-1.82-.63l-1.103.303c-.066.019-.176.011-.299-.071a5.991 5.991 0 00-.668-.386c-.133-.066-.194-.158-.212-.224L10.16 1.29C9.99.645 9.444.095 8.701.031A8.094 8.094 0 008 0zm1.5 8a1.5 1.5 0 11-3 0 1.5 1.5 0 013 0zM11 8a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
                Settings
            </button>
            <div class="settings-popover" id="settings-popover">
                <div class="settings-section">
                    <div class="settings-section-label">Pair another device</div>
                    <p class="settings-description">Generate a code to pair another phone, tablet, or browser.</p>
                    <button class="settings-action-btn" id="generate-invite-btn" onclick="generateInviteCode()">
                        Generate pairing code
                    </button>
                    <div id="invite-result" class="invite-result"></div>
                </div>
                <div class="settings-divider"></div>
                <div class="settings-section">
                    <div class="settings-section-label">This device</div>
                    <p class="settings-description">Removes access from this browser. You'll need a new pairing code to reconnect.</p>
                    <button class="settings-danger-btn" onclick="clearPairing()">
                        Unpair this device
                    </button>
                </div>
            </div>
        </div>
    </div>
    <div class="container" id="sessions" data-layout="1" data-grouping="all"></div>

    <script>${dashboardJs}</script>
</body>
</html>`;
