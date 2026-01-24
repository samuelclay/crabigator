// Dashboard JavaScript - combined export
import { constantsJs } from './constants';
import { scrollbackJs } from './scrollback';
import { styleJs } from './style';
import { deployJs } from './deploy';
import { inputJs } from './input';
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
import { sseJs } from './sse';
import { pairingJs } from './pairing';
import { initJs } from './init';

export const dashboardJs = [
    constantsJs,
    scrollbackJs,
    styleJs,
    deployJs,
    inputJs,
    layoutJs,
    ansiJs,
    utilsJs,
    sessionJs,
    sessionStateJs,
    formatJs,
    statsWidgetJs,
    gitWidgetJs,
    changesWidgetJs,
    eventsJs,
    promptJs,
    sseJs,
    pairingJs,
    initJs
].join('\n');
