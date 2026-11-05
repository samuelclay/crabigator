"""Offline checks for the sample host. No request reaches the live service."""
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import AsyncMock, Mock, patch
import urllib.error

from host import Host, Session


class SampleHostTests(unittest.IsolatedAsyncioTestCase):
    def session(self):
        session = Session(None, 'claude', 0)
        session.send = AsyncMock()
        return session

    async def test_commands_and_terminal_controls_are_inert(self):
        session = self.session()
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / 'must-not-exist'
            await session.command(f'$(touch {marker})\x1b[31m')
            self.assertFalse(marker.exists())
            self.assertNotIn('\x1b', session.transcript)
            self.assertIn('No commands were executed.', session.transcript)

    async def test_question_choice_and_custom_reply_follow_native_protocol(self):
        session = self.session()
        await session.command('question')
        self.assertEqual(session.prompt['cursor_row'], 1)
        await session.receive({'type': 'key_sequence', 'steps': [{'type': 'text', 'text': '2'}]})
        self.assertIn('You: Light\n', session.transcript)
        self.assertIsNone(session.prompt)
        await session.command('question')
        await session.receive({'type': 'key_sequence', 'steps': [
            {'type': 'text', 'text': '3'}, {'type': 'text', 'text': '1'},
            {'type': 'key', 'key': 'enter'},
        ]})
        self.assertIn('You: 1\n', session.transcript)
        self.assertNotIn('You: 31enter', session.transcript)

    async def test_prompt_reply_does_not_start_another_command(self):
        session = self.session()
        await session.command('question')
        await session.receive({'type': 'key_sequence', 'steps': [
            {'type': 'text', 'text': '3'}, {'type': 'text', 'text': 'notify'},
            {'type': 'key', 'key': 'enter'},
        ]})
        self.assertEqual(session.state, 'complete')
        self.assertIsNone(session.notification_at)

    async def test_stats_start_time_uses_unix_seconds(self):
        session = self.session()
        await session.publish()
        stats = next(call.kwargs for call in session.send.call_args_list if call.kwargs['type'] == 'stats')
        self.assertLess(abs(stats['session_start'] - time.time()), 5)

    async def test_setup_retries_transient_errors_without_logging_their_contents(self):
        with tempfile.TemporaryDirectory() as directory:
            host = Host(Path(directory), 'https://example.test')
            host.setup = Mock(side_effect=[urllib.error.URLError('private value'), None])
            host.start_session = Mock()
            event = Mock(wait=AsyncMock())
            with patch('host.asyncio.sleep', new_callable=AsyncMock) as sleep, \
                    patch('host.asyncio.Event', return_value=event), self.assertLogs('store-review') as logs:
                await host.run()
            sleep.assert_awaited_once_with(1)
            self.assertEqual(host.setup.call_count, 2)
            self.assertEqual(host.start_session.call_count, 2)
            self.assertNotIn('private value', '\n'.join(logs.output))


if __name__ == '__main__':
    unittest.main()
