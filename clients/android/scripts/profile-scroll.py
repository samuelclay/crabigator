"""Capture a repeatable foreground scroll with Android frame stats and Perfetto.

Open the desired session/list first. This never signs out, clears app data, or
changes device settings. Trace files stay local and may contain session titles.
"""
import argparse
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--serial', required=True)
parser.add_argument('--label', required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--adb', default='adb')
parser.add_argument('--x', type=int, default=250, help='Horizontal swipe coordinate: list or terminal pane')
parser.add_argument('--start-down', action='store_true', help='Start toward older output in a bottom-pinned terminal')
parser.add_argument('--top', type=int, default=260)
parser.add_argument('--bottom', type=int, default=960)
args = parser.parse_args()
if not args.label.replace('-', '').replace('_', '').isalnum():
    parser.error('--label must contain only letters, numbers, hyphens, and underscores')
args.output.mkdir(parents=True, exist_ok=True)
package = 'com.crabigator.app'


def adb(*command):
    return subprocess.check_output([args.adb, '-s', args.serial, *command], text=True)


def check_focus():
    focus = next((line for line in adb('shell', 'dumpsys', 'window').splitlines() if 'mCurrentFocus=' in line), '')
    if package + '/' not in focus:
        raise RuntimeError('Crabigator left the foreground. Discard this run: ' + focus)


config = '''buffers { size_kb: 32768 fill_policy: RING_BUFFER }
duration_ms: 18000
data_sources { config { name: "linux.ftrace" ftrace_config {
 ftrace_events: "sched/sched_switch"
 ftrace_events: "sched/sched_waking"
 atrace_categories: "gfx"
 atrace_categories: "view"
 atrace_categories: "wm"
 atrace_categories: "am"
 atrace_categories: "dalvik"
 atrace_apps: "com.crabigator.app"
} } }
data_sources { config { name: "linux.process_stats" process_stats_config { scan_all_processes_on_start: true } } }
data_sources { config { name: "android.surfaceflinger.frametimeline" } }
'''
check_focus()
adb('shell', 'dumpsys', 'gfxinfo', package, 'reset')
remote = f'/data/misc/perfetto-traces/crabigator-{args.label}.perfetto-trace'
# Send config on stdin: Samsung denies Perfetto access to /data/local/tmp.
with subprocess.Popen([args.adb, '-s', args.serial, 'shell', 'perfetto', '--txt', '-c', '-', '-o', remote],
                      stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT) as trace:
    trace.stdin.write(config.encode())
    trace.stdin.close()
    trace.stdin = None
    try:
        time.sleep(1)
        for i in range(8):
            check_focus()
            start, end = (args.bottom, args.top) if (i % 4 < 2) != args.start_down else (args.top, args.bottom)
            adb('shell', 'input', 'swipe', str(args.x), str(start), str(args.x), str(end), '700')
            time.sleep(.25)
        check_focus()
        stats = adb('shell', 'dumpsys', 'gfxinfo', package, 'framestats')
        if 'Total frames rendered: 0\n' in stats:
            raise RuntimeError('No app frames rendered; discard this run')
        (args.output / f'{args.label}-gfx.txt').write_text(stats)
        log = trace.communicate(timeout=25)[0]
        (args.output / f'{args.label}-capture.txt').write_bytes(log)
        if trace.returncode:
            raise RuntimeError(log.decode())
        adb('pull', remote, str(args.output / f'{args.label}.perfetto-trace'))
        adb('shell', 'rm', remote)
        print('\n'.join(stats.splitlines()[:21]))
    finally:
        if trace.poll() is None:
            trace.terminate()
