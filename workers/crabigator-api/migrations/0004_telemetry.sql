-- Telemetry table for tracking update checks and usage
CREATE TABLE IF NOT EXISTS telemetry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    machine_name TEXT,
    os TEXT,
    timezone_offset INTEGER,
    app_version TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_telemetry_device ON telemetry(device_id);
CREATE INDEX IF NOT EXISTS idx_telemetry_created ON telemetry(created_at);
CREATE INDEX IF NOT EXISTS idx_telemetry_version ON telemetry(app_version);
