-- Device Groups for Multi-Desktop Linking
-- Migration: 0002_device_groups
--
-- Problem: Users with multiple desktops (work laptop, home desktop) want a single
-- dashboard showing all sessions.
--
-- Solution: Device groups allow multiple desktops to be linked through a single
-- mobile device pairing.

-- Device groups - each group can contain multiple desktops
CREATE TABLE IF NOT EXISTS device_groups (
    id TEXT PRIMARY KEY,                      -- group_id (UUID)
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Add group_id column to devices table
-- This is nullable - devices without a group haven't been paired yet
ALTER TABLE devices ADD COLUMN group_id TEXT REFERENCES device_groups(id);

-- Create index for efficient group lookups
CREATE INDEX IF NOT EXISTS idx_devices_group ON devices(group_id) WHERE group_id IS NOT NULL;

-- Note: When a mobile pairs with a desktop:
-- 1. If desktop has no group_id, create a new group and assign it
-- 2. If desktop already has a group_id, the mobile joins that group
-- 3. When same mobile pairs with another desktop, that desktop joins the existing group
