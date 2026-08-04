// Dashboard CSS barrel - combines all CSS module fragments
import { baseCss } from './base';
import { deployCss } from './deploy';
import { headerCss } from './header';
import { sessionCardCss } from './session-card';
import { recapCss } from './recap';
import { terminalCss } from './terminal';
import { widgetsCss } from './widgets';
import { inputCss } from './input';
import { popoversCss } from './popovers';
import { promptCss } from './prompt';
import { pairingCss } from './pairing';
import { layoutCss } from './layout';
import { sidebarCss } from './sidebar';
import { paywallCss } from './paywall';
import { giftCss } from './gift';
import { voiceCss } from './voice';
import { prBoardCss } from './pr-board';

export const dashboardCss = [
    baseCss,
    deployCss,
    headerCss,
    sessionCardCss,
    recapCss,
    terminalCss,
    widgetsCss,
    inputCss,
    popoversCss,
    promptCss,
    pairingCss,
    layoutCss,
    sidebarCss,
    paywallCss,
    giftCss,
    voiceCss,
    prBoardCss,
].join('\n');
