-- Add os_version and cli_version columns to telemetry table
ALTER TABLE telemetry ADD COLUMN os_version TEXT;
ALTER TABLE telemetry ADD COLUMN cli_version TEXT;
