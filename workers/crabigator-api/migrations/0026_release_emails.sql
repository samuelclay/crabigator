-- Release emails: each release's highlights, emailed to the homepage's
-- newsletter subscribers.

-- The link in every email that unsubscribes this address, without a login.
ALTER TABLE email_signups ADD COLUMN unsubscribe_token TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_email_signups_unsubscribe ON email_signups(unsubscribe_token);

-- One row per release email. `make announce-release` adds the tag; the
-- Worker's five-minute cron fills in the notes from GitHub and sends them to
-- each subscriber in batches.
CREATE TABLE IF NOT EXISTS release_emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tag TEXT NOT NULL,
    -- A test: only this address gets the email.
    test_recipient TEXT,
    subject TEXT,
    html TEXT,
    text TEXT,
    -- Every subscriber up to this id has been sent the email.
    last_signup_id INTEGER NOT NULL DEFAULT 0,
    sent_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    -- When a cron run took this email, so two runs don't send it at once.
    claimed_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    finished_at INTEGER
);

-- Each release goes to the list once; tests can repeat.
CREATE UNIQUE INDEX IF NOT EXISTS idx_release_emails_tag ON release_emails(tag) WHERE test_recipient IS NULL;
