#!/usr/bin/env python3
"""Collect local user prompts and the Android port's commit trail. No credentials or assistant/tool output."""
import datetime, json, pathlib, subprocess
root = pathlib.Path(__file__).resolve().parents[3]
out = root / 'clients/ios/docs'
cutoff = '2026-10-07'
records = []
for path in (pathlib.Path.home() / '.codex/sessions').rglob('*.jsonl'):
    if datetime.datetime.fromtimestamp(path.stat().st_mtime).isoformat() < cutoff:
        continue
    try:
        with path.open() as f:
            first = json.loads(next(f))
            meta = first.get('payload', {})
            if pathlib.Path(meta.get('cwd', '/')) != root and root not in pathlib.Path(meta.get('cwd', '/')).parents:
                continue
            if isinstance(meta.get('source'), dict) and 'subagent' in meta['source']:
                continue
            for line in f:
                try: row = json.loads(line)
                except json.JSONDecodeError: continue
                if row.get('timestamp', '') < cutoff: continue
                p = row.get('payload', {})
                if row.get('type') == 'response_item' and p.get('type') == 'message' and p.get('role') == 'user':
                    message = '\n'.join(c.get('text', '') for c in p.get('content', []) if c.get('type') in ('input_text', 'text'))
                    if message and not message.startswith(('# AGENTS.md', '<environment_context>', '<INSTRUCTIONS>')):
                        records.append((row.get('timestamp', ''), meta.get('id', path.stem), str(path), message))
    except (OSError, StopIteration, json.JSONDecodeError): continue
records = sorted({(timestamp, message): (timestamp, session, path, message) for timestamp, session, path, message in records}.values())
with (out / 'user-prompts.local.md').open('w') as f:
    f.write('# Crabigator user prompts since October 7, 2026\n\nLocal research copy; excluded from Git. User messages only.\n\n')
    for timestamp, session, path, message in records:
        f.write(f'## {timestamp} · {session}\n\nSource: `{path}`\n\n{message}\n\n')
commits = subprocess.check_output(['git','log','--reverse',f'--since={cutoff}','--format=%H','--','clients/android'], cwd=root, text=True).splitlines()
with (out / 'android-commits.md').open('w') as f:
    f.write('# Android commits reviewed for the iOS port\n\nChronological history. The current Android sources determine the final behavior when later commits revise earlier choices.\n\n')
    for sha in commits:
        f.write(subprocess.check_output(['git','show','--format=fuller','--stat',sha,'--','clients/android'],cwd=root,text=True)+'\n')
print(f'{len(records)} user prompts; {len(commits)} Android commits')
