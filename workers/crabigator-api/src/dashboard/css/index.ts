// Dashboard CSS barrel - combines all CSS module fragments
import { baseCss } from './base';
import { deployCss } from './deploy';
import { headerCss } from './header';
import { sessionCardCss } from './session-card';
import { terminalCss } from './terminal';
import { widgetsCss } from './widgets';
import { inputCss } from './input';
import { popoversCss } from './popovers';
import { promptCss } from './prompt';
import { pairingCss } from './pairing';
import { layoutCss } from './layout';
import { paywallCss } from './paywall';
import { giftCss } from './gift';

export const dashboardCss = [
    baseCss,
    deployCss,
    headerCss,
    sessionCardCss,
    terminalCss,
    widgetsCss,
    inputCss,
    popoversCss,
    promptCss,
    pairingCss,
    layoutCss,
    paywallCss,
    giftCss,
].join('\n');
