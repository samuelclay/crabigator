-- Every voice transcript, kept so a transcript that never reached its session
-- can still be copied later. Kept independently of session and device deletion.
-- recorded_seconds is the app's own timer; audio_seconds is the length of the
-- audio the server received. A gap between them means the recording lost audio
-- before upload.
CREATE TABLE voice_transcripts (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL,
    session_id TEXT,
    session_title TEXT,
    client TEXT,
    text TEXT NOT NULL,
    recorded_seconds REAL,
    audio_seconds REAL,
    created_at INTEGER NOT NULL
);
CREATE INDEX voice_transcripts_group ON voice_transcripts(group_id);
CREATE INDEX voice_transcripts_session ON voice_transcripts(group_id, session_id);
