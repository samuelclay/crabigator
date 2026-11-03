-- Keep display names separate from hostnames so old clients cannot overwrite them.
ALTER TABLE devices ADD COLUMN display_name TEXT;
