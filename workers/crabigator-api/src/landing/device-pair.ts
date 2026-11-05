// A working session on the desktop with the same session beside it in the
// iPhone app. Rows marked `stream` arrive on the desktop, then on the phone,
// and both flow scenes move (see hero-stream.ts and the flow-frame styles).
import { iconMicrophone, iconKeyboard, iconWifi } from './icons';
import { appModulesHtml, type SessionValues } from './app-modules';

interface PairCapture {
    cols: number;
    desktopHtml: string;
    phoneHtml: string;
    session: SessionValues;
}

export function devicePairHtml(options: {
    capture: PairCapture;
    command: string;
    title: string;
    phoneFlow: string;
    label: string;
    reverse?: boolean;
}): string {
    const { capture, command, title, phoneFlow, label, reverse } = options;
    return `
            <figure class="device-pair agent-pair${reverse ? ' reverse' : ''}" data-step="380" aria-label="${label}">
                <div class="desktop-frame pair-desktop">
                    <div class="desktop-bar"><span class="window-dots" aria-hidden="true"><i></i><i></i><i></i></span><span>crabigator — ${command}</span></div>
                    <div class="pair-capture term-capture" style="--cols:${capture.cols}" aria-hidden="true">${capture.desktopHtml}</div>
                </div>
                <div class="phone-frame pair-phone">
                    <div class="phone-screen" aria-hidden="true">
                        <div class="phone-status"><span>9:41</span><span class="phone-island"></span><span class="phone-indicators"><span class="signal-bars"><i></i><i></i><i></i><i></i></span>${iconWifi}<span class="phone-battery"></span></span></div>
                        <div class="phone-app-bar"><span class="phone-back">‹</span><div><strong>${title}</strong><span>ClayBook · Live</span></div><span class="phone-type">ᴀA</span></div>
                        <div class="phone-terminal">${capture.phoneHtml}</div>
                        ${appModulesHtml(capture.session, phoneFlow)}
                        <div class="phone-composer">${iconMicrophone}${iconKeyboard}<span>Message</span></div>
                        <div class="phone-home"><span></span></div>
                    </div>
                </div>
                <button class="stream-toggle" type="button" aria-label="Pause animation" hidden>Ⅱ</button>
            </figure>`;
}
