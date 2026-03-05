-- Store session title history (JSON array of OSC terminal titles)
ALTER TABLE sessions ADD COLUMN titles TEXT;
