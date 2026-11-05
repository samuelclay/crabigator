// Two short sections: four plain claims, then cards whose screenshots carry
// the detail (real terminal strips, real scene frames, and the MCP example).
import { cardCaptures } from './desktop-capture';
import { flowScenes } from './flow-scenes';
import { toolExamples } from '../mcp/examples';
import { highlightJson } from '../mcp/docs';

const claims = [
    ['Your terminal, untouched.', 'The agent runs on your computer, in its own terminal. Crabigator adds a status bar below it.'],
    ['Every computer at once.', 'Sessions from all your computers show up together.'],
    ['Survives a dropped connection.', 'Events wait offline and send when the connection returns.'],
    ['Free and open source.', 'MIT licensed and written in Rust. Use the hosted dashboard or run your own.'],
];

export const claimsHtml = `
        <section class="section ownership" id="security" aria-labelledby="ownership-title">
            <h2 id="ownership-title">Still your terminal.<span>Still your machine.</span></h2>
            <div class="claims">${claims.map(([title, detail]) => `<div><strong>${title}</strong><p>${detail}</p></div>`).join('')}</div>
            <a class="text-link" href="https://github.com/samuelclay/crabigator#readme" target="_blank" rel="noopener">Read the source</a>
        </section>`;

const scenes = Object.entries(flowScenes).filter(([name]) => name.startsWith('mosaic_')).map(([, frames]) => `<span class="scene-frames">${frames}</span>`).join('');
const waiting = toolExamples.wait_for_attention;
const waitingSession = (waiting.output as { sessions: { title: string; state: string; device_name: string }[] }).sessions[0];

function card(shot: string, title: string, detail: string): string {
    return `<article class="card"><div class="card-shot" aria-hidden="true">${shot}</div><strong>${title}</strong><p>${detail}</p></article>`;
}

export const cardsHtml = `
        <section class="section extras" id="more" aria-labelledby="extras-title">
            <h2 id="extras-title">Every session turbocharged with context.<span>Recaps, pull requests, scenes, and an MCP server.</span></h2>
            <div class="cards">
                ${card(`<div class="card-term term-capture" style="--cols:${cardCaptures.recap.cols}">${cardCaptures.recap.html}</div>`,
                    'A recap after every turn.', 'Read what changed without scrolling back.')}
                ${card(`<div class="card-term term-capture" style="--cols:${cardCaptures.prs.cols}">${cardCaptures.prs.html}</div>`,
                    'Pull requests, tracked.', 'Each session lists the pull requests it touched, with their status.')}
                ${card(`<div class="card-scenes">${scenes}</div>`,
                    'A scene for every session.', 'Fifteen scenes, each moving with the work. <a href="https://github.com/robdmac/flow" target="_blank" rel="noopener" title="flow by Rob Macrae, on GitHub">Adapted from flow</a>')}
                ${card(`<div class="card-mcp"><code>wait_for_attention ${JSON.stringify(waiting.args)}</code><pre>${highlightJson({
                    timed_out: false,
                    sessions: [{ title: waitingSession.title, state: waitingSession.state, device_name: waitingSession.device_name }],
                })}</pre></div>`,
                    'An MCP server for your agents.', 'Any MCP client can read sessions and answer prompts. <a href="/mcp-tools">See the tools</a>')}
            </div>
        </section>`;
