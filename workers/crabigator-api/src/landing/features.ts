// Two short sections: four plain claims, then cards whose screenshots carry
// the detail (a captured recap, example PR states, real scenes, and MCP output).
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

// Reflow the captured three-line recap so its terminal padding doesn't dictate the type size.
const recapPreview = `<div class="feature-recap">
    <div>Mapped night dial usage across 13 scenes</div>
    <ul><li>7 scenes respond to night: balloon, rocket, bubbles, ski, surf, train, roadster</li>
        <li>6 scenes ignore night setting: avalon, doom, earthrise, engine, fire, warp</li></ul>
    <aside><b>✻ Did you know?</b> In the train scene, birds stop appearing after dark.</aside>
</div>`;

// An illustrative tracking strip, including open PRs in different merge conditions.
const prExamples = [
    { number: 6, adds: 930, deletes: 247, files: 22, state: 'Merged' },
    { number: 5, adds: 278, deletes: 114, files: 3, state: 'Open' },
    { number: 7, adds: 84, deletes: 19, files: 5, state: 'Conflicts' },
    { number: 8, adds: 46, deletes: 12, files: 2, state: 'Behind' },
];
const prPreview = `<div class="feature-prs">
    <div class="feature-pr-header"><b>●</b> Tracking four pull requests <small>just now &nbsp; Δ ·</small></div>
    ${prExamples.map(pr => `<div class="feature-pr">
        <span class="feature-pr-id">★ crabigator <b>#${pr.number}</b></span>
        <span class="feature-pr-status pr-${pr.state.toLowerCase()}">${pr.state}</span>
        <span class="feature-pr-diff"><b>+${pr.adds}</b> <i>−${pr.deletes}</i> <small>${pr.files} files</small></span>
        <span class="feature-pr-actions">↓ &nbsp;✕</span>
    </div>`).join('')}
</div>`;

function card(shot: string, title: string, detail: string, previewLabel?: string): string {
    const accessibility = previewLabel ? `role="group" aria-label="${previewLabel}"` : 'aria-hidden="true"';
    return `<article class="card"><div class="card-shot" ${accessibility}>${shot}</div><strong>${title}</strong><p>${detail}</p></article>`;
}

export const cardsHtml = `
        <section class="section extras" id="more" aria-labelledby="extras-title">
            <h2 id="extras-title">Every session turbocharged with context.<span>Recaps, pull requests, scenes, and an MCP server.</span></h2>
            <div class="cards">
                ${card(recapPreview, 'A recap after every turn.', 'Read what changed without scrolling back.', 'Example turn recap')}
                ${card(prPreview, 'Pull requests, tracked.', 'Each session lists the pull requests it touched, with their status.', 'Example pull request tracker')}
                ${card(`<div class="card-scenes">${scenes}</div>`,
                    'A scene for every session.', 'Fifteen scenes, each moving with the work. <a href="https://github.com/robdmac/flow" target="_blank" rel="noopener" title="flow by Rob Macrae, on GitHub">Adapted from flow</a>')}
                ${card(`<div class="card-mcp"><code>wait_for_attention ${JSON.stringify(waiting.args)}</code><pre>${highlightJson({
                    timed_out: false,
                    sessions: [{ title: waitingSession.title, state: waitingSession.state, device_name: waitingSession.device_name }],
                })}</pre></div>`,
                    'An MCP server for your agents.', 'Any MCP client can read sessions and answer prompts. <a href="/mcp-tools">See the tools</a>')}
            </div>
        </section>`;
