#!/usr/bin/env python3
"""Serve clearly labeled sample sessions through the normal Crabigator API."""
import argparse
import asyncio
import hashlib
import hmac
import json
import logging
import os
from pathlib import Path
import secrets
import time
import urllib.error
import urllib.request
from urllib.parse import urlsplit
import uuid

from websockets.asyncio.client import connect

LOG = logging.getLogger('store-review')
HELP = ('CRABIGATOR LIVE REVIEW — SAMPLE DATA\n'
        'Messages here never execute commands or access files.\n'
        'Try: question, permission, notify, reset.\n'
        'notify waits 20 seconds so you can background the app.\n'
        'The microphone uses the normal live transcription service.\n')


def now():
    return int(time.time() * 1000)


def save_private(path, value):
    temporary = path.with_suffix('.tmp')
    with os.fdopen(os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), 'w') as output:
        json.dump(value, output, indent=2)
    temporary.replace(path)


class Host:
    def __init__(self, directory, origin):
        self.directory = directory
        directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.identity_path = directory / 'identity.json'
        if self.identity_path.exists():
            self.identity = json.loads(self.identity_path.read_text())
            if self.identity['origin'] != origin:
                raise ValueError('Use a separate state directory for each origin')
        else:
            self.identity = dict(origin=origin, device_id=str(uuid.uuid4()),
                                 secret_hash=hashlib.sha256(secrets.token_bytes(32)).hexdigest())
            save_private(self.identity_path, self.identity)
        self.origin = origin
        self.sessions = []
        self.tasks = set()

    def headers(self, method, path):
        stamp = str(now())
        signature = hmac.new(self.identity['secret_hash'].encode(),
                             f'{method}:{path}:{stamp}'.encode(), hashlib.sha256).hexdigest()
        return {'X-Device-Id': self.identity['device_id'], 'X-Timestamp': stamp,
                'X-Signature': signature, 'Content-Type': 'application/json'}

    def request(self, path, body, authenticated=True):
        headers = self.headers('POST', path) if authenticated else {'Content-Type': 'application/json'}
        headers['User-Agent'] = 'Crabigator-Store-Review/1.0'
        request = urllib.request.Request(self.origin + path, json.dumps(body).encode(), headers)
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)

    def setup(self):
        self.request('/api/devices', {**{key: self.identity[key] for key in ('device_id', 'secret_hash')},
                                     'name': 'Crabigator Store Review (sample data)'}, False)
        if 'pairing_code' not in self.identity:
            pairing = self.request('/api/pairing/generate', {})
            self.identity['pairing_code'] = pairing['code']
            self.identity['pairing_expires_at'] = pairing['expires_at']
            save_private(self.identity_path, self.identity)
        LOG.info('Review identity ready; pairing code is in the private identity file')

    def start_session(self, platform, index):
        session = Session(self, platform, index)
        self.sessions.append(session)
        task = asyncio.create_task(session.run())
        self.tasks.add(task)
        task.add_done_callback(self.tasks.discard)

    async def run(self):
        delay = 1
        while True:
            try:
                await asyncio.to_thread(self.setup)
                break
            except (urllib.error.URLError, TimeoutError) as error:
                if isinstance(error, urllib.error.HTTPError) and error.code != 429 and error.code < 500:
                    raise
                LOG.warning('Review setup retrying (%s)', type(error).__name__)
                await asyncio.sleep(delay)
                delay = min(delay * 2, 60)
        self.start_session('claude', 0)
        self.start_session('codex', 1)
        await asyncio.Event().wait()


class Session:
    def __init__(self, host, platform, index):
        self.host, self.platform, self.index = host, platform, index
        self.started = int(time.time())
        self.turns = 0
        self.state = 'ready'
        self.prompt = None
        self.transcript = HELP
        self.notification_at = None
        self.session_id = None
        self.title = f'Live review sample {index + 1} · {"Add dark mode" if index == 0 else "Improve search"}'

    async def send(self, **event):
        await self.ws.send(json.dumps(event))

    async def snapshot(self):
        await self.send(type='title', title=self.title)
        await self.send(type='git', repo_owner='sample', repo_name='website',
                        branch='sample/dark-mode' if self.index == 0 else 'sample/search',
                        files=[dict(path='src/theme.ts', status='M ', additions=18, deletions=4)])
        await self.send(type='changes', by_language=[dict(language='TypeScript', changes=[dict(
            kind='Function', name='applyTheme', change_type='modified', additions=18, deletions=4,
            file_path='src/theme.ts', line_number=12)])])
        await self.send(type='flow', scene='rain', seed=self.index + 17, level=3, waiting=False,
                        night=False, tint='normal', boost=0)
        await self.send(type='prs', prs=[dict(number=23 + self.index, owner='sample', repo='website',
            url='', branch='sample/review', title=self.title, state='OPEN', is_draft=False,
            additions=18, deletions=4, changed_files=1, mergeable='MERGEABLE', merge_state_status='CLEAN',
            checks_passed=3, checks_failed=0, checks_pending=0, checks_total=3,
            created_here=True, primary=True, refreshed_at=now())])
        await self.publish()

    async def publish(self):
        await self.send(type='screen', content=self.transcript)
        await self.send(type='scrollback_history', content=self.transcript)
        await self.send(type='stats', prompts=self.turns, completions=self.turns, tools=3,
                        compressions=0, thinking_seconds=12, work_seconds=30, model='Sample agent',
                        tool_timestamps=[], session_start=self.started)
        await self.send(type='prompt', prompt=self.prompt)
        await self.send(type='state', state=self.state, timestamp=now())

    async def command(self, text, choice=True):
        # Never treat reviewer input as a path, shell command, or Python expression.
        text = ''.join(char for char in str(text) if char.isprintable() or char == '\n')[:2000]
        command = text.strip().lower()
        if self.prompt:
            options = (self.prompt['questions'][0]['options']
                       if self.prompt['prompt_type'] == 'question' else self.prompt['options'])
            if choice:
                text = next((option['label'] for option in options if option['value'] == text.strip()), text)
            # A prompt reply is an answer, even if it spells a sample command.
            command = ''
        self.turns += 1
        self.prompt = None
        self.state = 'complete'
        self.notification_at = None
        if command == 'reset':
            self.transcript = HELP
        else:
            self.transcript = (self.transcript + '\nYou: ' + text + '\n')[-16000:]
        if command in ('question', 'notify'):
            if command == 'notify':
                self.notification_at = time.monotonic() + 20
                self.state = 'thinking'
                self.transcript += 'Sample agent: Background the app now. A question arrives in 20 seconds.\n'
            else:
                self.show_question()
        elif command == 'permission':
            self.state = 'permission'
            self.prompt = dict(prompt_type='permission', tool_name='Sample test run',
                tool_input='This action only changes sample output.', selected_option=1,
                options=[dict(label='Allow sample test', value='1'), dict(label='Decline', value='2')])
            self.transcript += 'Sample agent: Allow this simulated test run?\n'
        elif command != 'reset':
            self.transcript += 'Sample agent: Received. Sample tests pass: 3 of 3. No commands were executed.\n'
        await self.publish()

    def show_question(self):
        self.state = 'question'
        self.prompt = dict(prompt_type='question', current_question=0, cursor_row=1, questions=[dict(
            question='Which sample theme should we preview?', header='Sample theme',
            options=[dict(label='Dark', value='1'), dict(label='Light', value='2')],
            multi_select=False, allows_other=True)])
        self.transcript += 'Sample agent: Which theme should we preview? 1. Dark  2. Light\n'

    async def receive(self, message):
        kind = message.get('type')
        if kind == 'answer':
            await self.command(message.get('text', ''))
        elif kind == 'key':
            await self.command(f"[Key: {message.get('key', '')}]")
        elif kind == 'key_sequence':
            # Native clients type a choice's number, or the "Other" row number
            # followed by custom text and Enter. Key names are not answer text.
            parts = [step.get('text', '') for step in message.get('steps', []) if step.get('type') == 'text']
            choice = True
            if self.prompt and self.prompt['prompt_type'] == 'question' and len(parts) > 1:
                other = str(len(self.prompt['questions'][0]['options']) + 1)
                if parts[0] == other:
                    parts = parts[1:]
                    choice = False
            await self.command(''.join(parts) or '[Sample keyboard action]', choice=choice)
        elif kind == 'viewer_status':
            await self.snapshot()
        elif kind == 'spawn':
            if len(self.host.sessions) < 6:
                platform = message.get('platform', self.platform)
                if platform not in ('claude', 'codex', 'opencode', 'grok'):
                    platform = self.platform
                self.host.start_session(platform, len(self.host.sessions))
            else:
                await self.command('Six sample sessions are already available. Use any existing session.')

    async def run(self):
        delay = 1
        while True:
            try:
                registered = await asyncio.to_thread(self.host.request, '/api/sessions', dict(
                    client_session_id=str(uuid.uuid5(uuid.UUID(self.host.identity['device_id']), str(self.index))),
                    cwd=f'/sample/website-{self.index + 1}', platform=self.platform))
                self.session_id = registered['id']
                path = urlsplit(registered['ws_url']).path
                async with connect(registered['ws_url'], additional_headers=self.host.headers('GET', path),
                                   user_agent_header='Crabigator-Store-Review/1.0', max_size=65536) as self.ws:
                    await self.snapshot()
                    LOG.info('Sample session %s connected', self.index + 1)
                    delay = 1
                    heartbeat_at = 0
                    while True:
                        if time.monotonic() >= heartbeat_at:
                            await self.send(type='heartbeat', timestamp=now())
                            heartbeat_at = time.monotonic() + 15
                        if self.notification_at and time.monotonic() >= self.notification_at:
                            self.notification_at = None
                            self.show_question()
                            await self.publish()
                        try:
                            raw = await asyncio.wait_for(self.ws.recv(), timeout=1)
                        except asyncio.TimeoutError:
                            continue
                        await self.receive(json.loads(raw))
            except Exception as error:
                # Do not log incoming messages, credentials, or authenticated URLs.
                LOG.warning('Sample session %s reconnecting (%s)', self.index + 1, type(error).__name__)
                await asyncio.sleep(delay)
                delay = min(delay * 2, 60)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--state-dir', type=Path, default=Path.home() / '.crabigator/store-review')
    parser.add_argument('--origin', default='https://drinkcrabigator.com')
    args = parser.parse_args()
    if urlsplit(args.origin).scheme != 'https':
        parser.error('--origin must use HTTPS')
    logging.basicConfig(level=logging.INFO, format='%(asctime)s %(message)s')
    asyncio.run(Host(args.state_dir, args.origin.rstrip('/')).run())


if __name__ == '__main__':
    main()
