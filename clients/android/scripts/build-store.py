#!/usr/bin/env python3
"""Build a signed Play bundle without putting the signing password in arguments."""
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
env = os.environ.copy()
env.setdefault("CRABIGATOR_ANDROID_KEYSTORE", str(Path.home() / ".crabigator/signing/crabigator-upload.jks"))
env["CRABIGATOR_ANDROID_KEYSTORE"] = str(Path(env["CRABIGATOR_ANDROID_KEYSTORE"]).expanduser().resolve())
if not env.get("CRABIGATOR_ANDROID_STORE_PASSWORD") and sys.platform == "darwin":
    password = subprocess.run(
        ["security", "find-generic-password", "-s", "com.crabigator.android-upload", "-a", "crabigator-upload", "-w"],
        capture_output=True, text=True,
    )
    if password.returncode == 0:
        env["CRABIGATOR_ANDROID_STORE_PASSWORD"] = password.stdout.rstrip("\n")
if not env.get("CRABIGATOR_ANDROID_STORE_PASSWORD"):
    sys.exit("Set CRABIGATOR_ANDROID_STORE_PASSWORD or unlock the Crabigator Android upload key in macOS Keychain.")
if not Path(env["CRABIGATOR_ANDROID_KEYSTORE"]).is_file():
    sys.exit("Set CRABIGATOR_ANDROID_KEYSTORE to the existing Crabigator upload keystore.")
raise SystemExit(subprocess.call(
    [str(root / "gradlew"), ":app:testDebugUnitTest", ":app:lintRelease", ":app:bundleRelease"], cwd=root, env=env,
))
