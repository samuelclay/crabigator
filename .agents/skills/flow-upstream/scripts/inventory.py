#!/usr/bin/env python3
"""Inventory Flow changes without changing either checkout or its port ledger."""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path


def git(repo, *args):
    return subprocess.check_output(
        ['git', '-C', str(repo), *args], text=True, stderr=subprocess.PIPE
    ).strip()


def commit(repo, ref):
    return git(repo, 'rev-parse', '--verify', '--end-of-options', ref + '^{commit}')


def ancestor(repo, base, head):
    result = subprocess.run(
        ['git', '-C', str(repo), 'merge-base', '--is-ancestor', base, head],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise ValueError(f'{base} is not an ancestor of {head}; reconcile history before reviewing')


def hint(paths):
    # These route inspection, never decide whether to port a commit.
    result = set()
    for path in paths:
        if path in {'hooks/activity.ts', 'hooks/agents.ts', 'hooks/waits.ts', 'hooks/crew.ts', 'hooks/waiting.ts'}:
            result.add('activity or companions')
        elif path.startswith('hooks/') and path not in {'hooks/register.tsx', 'hooks/hooks.json', 'hooks/settings.ts', 'hooks/sessions.ts', 'hooks/picker.ts', 'hooks/svg.ts', 'hooks/sound.ts', 'hooks/soundscape.ts', 'hooks/sound-files.ts'}:
            result.add('scene or shared rendering')
        elif path == 'hooks/register.tsx' or path.endswith('/mapping.ts'):
            result.add('adapter events: inspect Rust equivalents')
        elif path.startswith('sounds/') or 'sound' in path or path.startswith('node/'):
            result.add('sound: check animation timing and RNG before excluding')
        elif path.startswith(('tests/', 'scripts/')):
            result.add('tests or tooling')
        else:
            result.add('host, settings, or documentation')
    return sorted(result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--flow-dir', type=Path, default=Path('../flow'))
    parser.add_argument('--crabigator-dir', type=Path, default=Path.cwd())
    parser.add_argument('--fetch', action='store_true')
    parser.add_argument('--base', help='Override review start; does not change any ledger')
    parser.add_argument('--head', default='upstream/main')
    args = parser.parse_args()
    flow = args.flow_dir.resolve()
    crab = args.crabigator_dir.resolve()
    url = git(flow, 'remote', 'get-url', 'upstream')
    if url.removesuffix('.git').rstrip('/') not in {
        'git@github.com:robdmac/flow', 'https://github.com/robdmac/flow',
        'ssh://git@github.com/robdmac/flow',
    }:
        raise ValueError(f'upstream must be robdmac/flow, found {url}')
    if args.fetch:
        subprocess.run(['git', '-C', str(flow), 'fetch', 'upstream'], check=True)
    fork = (crab / 'src/flow/FORK.md').read_text()
    match = re.search(r'\| Synced to \| `([0-9a-f]{7,40})`', fork)
    if not match:
        raise ValueError('Cannot read the Synced to SHA from src/flow/FORK.md')
    port_base = commit(flow, match[1])
    ledger_path = crab / 'src/flow/upstream-review.json'
    ledger = json.loads(ledger_path.read_text()) if ledger_path.exists() else {}
    if ledger and ledger.get('version') != 1:
        raise ValueError('Unsupported decision ledger version')
    decisions = ledger.get('decisions', [])
    allowed = {'needs-decision', 'selected', 'ported', 'already-present', 'skipped', 'deferred'}
    for decision in decisions:
        if not isinstance(decision, dict) or not isinstance(decision.get('id'), str) or not decision['id'].strip() or decision.get('status') not in allowed or not isinstance(decision.get('commits'), list) or not decision['commits']:
            raise ValueError(f'Invalid decision: {decision!r}')
    base = commit(flow, args.base or ledger.get('reviewed_through') or port_base)
    head = commit(flow, args.head)
    ancestor(flow, base, head)
    ancestor(flow, port_base, head)
    commits = []
    for sha in git(flow, 'rev-list', '--reverse', '--topo-order', f'{base}..{head}').splitlines():
        parents = git(flow, 'rev-list', '--parents', '-n', '1', sha).split()[1:]
        paths = git(flow, 'diff-tree', '--no-commit-id', '--name-only', '-r', parents[0], sha).splitlines()
        commits.append({
            'sha': sha, 'subject': git(flow, 'show', '-s', '--format=%s', sha),
            'date': git(flow, 'show', '-s', '--format=%cs', sha),
            'merge': len(parents) > 1, 'paths': paths, 'inspection_hints': hint(paths),
            'decisions': [d['id'] for d in decisions if sha in d['commits']],
        })
    fork_head = commit(flow, 'HEAD')
    common = git(flow, 'merge-base', fork_head, head)
    unfinished = [d for d in decisions if d['status'] in {'selected', 'deferred', 'needs-decision'}]
    print(json.dumps({
        'upstream': url, 'port_baseline': port_base, 'review_base': base,
        'upstream_head': head, 'commits': commits, 'prior_decisions': decisions,
        'unfinished_decisions': unfinished,
        'changed_files': git(flow, 'diff', '--stat', base, head),
        'custom_fork': {
            'head': fork_head, 'common_ancestor': common,
            'ahead_behind': git(flow, 'rev-list', '--left-right', '--count', f'{fork_head}...{head}'),
            'commits_not_in_upstream': git(flow, 'log', '--oneline', f'{head}..{fork_head}').splitlines(),
            'diff_from_upstream': git(flow, 'diff', '--stat', head, fork_head),
            'working_tree': git(flow, 'status', '--short'),
        },
        'note': 'Merge paths are relative to their first parent and overlap feature commits. Inspect merge resolution changes too; hints are not porting decisions.',
    }, indent=2))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        detail = exc.stderr.strip() if isinstance(exc, subprocess.CalledProcessError) and exc.stderr else str(exc)
        print(f'flow-upstream: {detail}', file=sys.stderr)
        sys.exit(1)
