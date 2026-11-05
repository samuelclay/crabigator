# Live store review environment

`host.py` runs a separate desktop identity with sample sessions. Reviewers pair through the normal app onboarding flow and use the production APIs, including voice transcription and push notifications. The host never starts an assistant, runs reviewer commands, or reads a project directory. It labels every session as sample data.

Keep this process running throughout both stores' review periods. Do not pair a personal device identity to it: pairing an already linked mobile identity merges its existing desktop group with the review group. Use a fresh install or an isolated test identity.

## Install on the review Mac

```sh
python3 -m venv ~/.crabigator/store-review/venv
~/.crabigator/store-review/venv/bin/pip install -r scripts/store-review/requirements.txt
~/.crabigator/store-review/venv/bin/python scripts/store-review/host.py
```

Run it under a launch agent with `RunAtLoad` and `KeepAlive` enabled. Use absolute paths and retain its private state directory across deployments. The state directory defaults to `~/.crabigator/store-review` and stores the separate device credential and reusable pairing code in `identity.json`. Its directory is private and the credential file is mode 0600. Put the code only in the stores' private review instructions, never in this repository, screenshots, or public descriptions.

Two sessions start automatically. Their IDs remain stable across restarts. A reviewer can create up to four more sample sessions through the app's normal new-session control. Network failures reconnect with backoff.

## Reviewer instructions

1. Enter the supplied pairing code on the app's initial pairing screen. No password, personal account, payment, or second computer is needed.
2. Open either **Live review sample** session. Send any text to see a live sample reply.
3. Send `question` to try selectable answers or `permission` to try approval controls.
4. Tap the microphone, allow microphone access, record a short message, and stop recording. Review and send the real transcription.
5. Allow notifications. Send `notify`, then background the app. A sample question arrives after 20 seconds. Open or expand the notification to answer it.
6. Try the PR board, terminal keyboard, session settings, and new-session control. All session data belongs to this separate sample environment.
7. Send `reset` to restore the help text. The offline **Try a demo** option remains available without pairing, but live voice and push tests use this environment.

Messages and transcripts in this shared review environment are visible to other reviewers who have its code. Use sample text only. Voice recordings go through the same transcription service described in the app's privacy policy.

## Verification and shutdown

Before submission, claim the code with a fresh mobile UUID and verify that the roster contains only sample sessions. Check a text reply, question/permission state changes, the PR board, real audio transcription, and a delayed notification on an isolated device installation. A successful provider response alone does not prove that a device displayed the notification.

To stop the deployed host on Studio:

```sh
launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/com.crabigator.store-review.plist
```

Retain the private identity while review remains active. To retire access afterward, revoke the review identity's pairing codes and linked devices through the normal API; stopping the process alone does not revoke credentials.
