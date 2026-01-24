// Dashboard HTML served at /dashboard
import { dashboardCss } from './dashboard/css';
import { dashboardJs } from './dashboard/js';

export const dashboardHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>Crabigator Dashboard</title>
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🦀</text></svg>">
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
