// Dashboard JavaScript - combined export
import { constantsJs } from './constants';
import { scrollbackJs } from './scrollback';
import { styleJs } from './style';
import { sessionsPopoverJs } from './sessions-popover';
import { deployJs } from './deploy';
import { inputJs } from './input';
import { keyboardJs } from './keyboard';
import { layoutJs } from './layout';
import { ansiJs } from './ansi';
import { utilsJs } from './utils';
import { sessionJs } from './session';
import { sessionStateJs } from './session-state';
import { formatJs } from './format';
import { statsWidgetJs } from './stats-widget';
import { gitWidgetJs } from './git-widget';
import { changesWidgetJs } from './changes-widget';
import { eventsJs } from './events';
import { promptJs } from './prompt';
import { voiceJs } from './voice';
import { sseJs } from './sse';
import { viewerActivityJs } from './viewer-activity';
import { pairingJs } from './pairing';
import { giftClaimJs } from './gift-claim';
import { paywallJs } from './paywall';
import { initJs } from './init';

export const dashboardJs = [
    constantsJs,
    scrollbackJs,
    styleJs,
    sessionsPopoverJs,
    deployJs,
    inputJs,
    keyboardJs,
    layoutJs,
    ansiJs,
    utilsJs,
    sessionJs,
    sessionStateJs,
    formatJs,
    statsWidgetJs,
    gitWidgetJs,
    changesWidgetJs,
    viewerActivityJs,  // Must be before eventsJs (defines sendViewerHeartbeat)
    eventsJs,
    promptJs,
    voiceJs,
    sseJs,
    pairingJs,
    giftClaimJs,  // Must be after pairingJs (uses isPaired and getAuthHeaders)
    paywallJs,  // Must be after pairingJs (uses auth functions)
    initJs
].join('\n');
