-- Website Analytics System Migration
-- Migration: 0007_analytics
--
-- Tracks landing page visitors, ad campaigns, product funnel, and NPM downloads.
-- Privacy-conscious: no PII stored unless user provides email.

-- Page view events for landing page analytics
CREATE TABLE IF NOT EXISTS page_views (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Visitor identification (privacy-conscious UUIDs from cookies)
    visitor_id TEXT NOT NULL,              -- UUID from localStorage (persistent)
    session_id TEXT NOT NULL,              -- UUID from sessionStorage (30min timeout)

    -- Page/request info
    page TEXT NOT NULL DEFAULT '/',        -- URL path
    referrer TEXT,                         -- Full referrer URL
    referrer_domain TEXT,                  -- Extracted domain from referrer

    -- UTM campaign tracking
    utm_source TEXT,                       -- utm_source param (e.g., 'google', 'twitter')
    utm_medium TEXT,                       -- utm_medium param (e.g., 'cpc', 'social')
    utm_campaign TEXT,                     -- utm_campaign param (e.g., 'launch2024')
    utm_content TEXT,                      -- utm_content param (ad variant)
    utm_term TEXT,                         -- utm_term param (keywords)

    -- Promo/offer code (accepts ?promo=, ?offer=, or ?code=)
    promo_code TEXT,

    -- Device/browser info (derived from User-Agent)
    device_type TEXT,                      -- 'desktop' | 'mobile' | 'tablet'
    browser TEXT,                          -- 'Chrome' | 'Safari' | 'Firefox' | etc.
    os TEXT,                               -- 'macOS' | 'Windows' | 'iOS' | 'Android' | etc.

    -- Geographic info (from Cloudflare headers, IP not stored)
    country TEXT,                          -- CF-IPCountry header (e.g., 'US')
    region TEXT,                           -- CF-Region header (e.g., 'California')
    city TEXT,                             -- CF-City header (e.g., 'San Francisco')

    -- Engagement metrics (updated via beacon on page unload)
    scroll_depth INTEGER DEFAULT 0,        -- Max scroll percentage (0-100)
    time_on_page INTEGER DEFAULT 0,        -- Seconds spent on page

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_page_views_visitor ON page_views(visitor_id);
CREATE INDEX IF NOT EXISTS idx_page_views_session ON page_views(session_id);
CREATE INDEX IF NOT EXISTS idx_page_views_created ON page_views(created_at);
CREATE INDEX IF NOT EXISTS idx_page_views_utm ON page_views(utm_source, utm_campaign);
CREATE INDEX IF NOT EXISTS idx_page_views_referrer ON page_views(referrer_domain);
CREATE INDEX IF NOT EXISTS idx_page_views_country ON page_views(country);

-- Click and interaction events
CREATE TABLE IF NOT EXISTS analytics_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Link to visitor/session
    visitor_id TEXT NOT NULL,
    session_id TEXT NOT NULL,

    -- Event info
    event_type TEXT NOT NULL,              -- 'click' | 'signup' | 'copy' | 'scroll_milestone'
    event_target TEXT,                     -- Element identifier (e.g., 'install_cta', 'copy_install')
    event_value TEXT,                      -- Additional value (e.g., email for signup)

    -- Page context
    page TEXT NOT NULL DEFAULT '/',

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_analytics_events_visitor ON analytics_events(visitor_id);
CREATE INDEX IF NOT EXISTS idx_analytics_events_type ON analytics_events(event_type, created_at);
CREATE INDEX IF NOT EXISTS idx_analytics_events_created ON analytics_events(created_at);

-- Product funnel tracking
-- Links visitor_id (browser) -> device_id (npm install) -> group_id (subscription)
CREATE TABLE IF NOT EXISTS funnel_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Identity chain (only one will be set per stage)
    visitor_id TEXT,                       -- Browser cookie (from landing page visit)
    device_id TEXT,                        -- Desktop device_id (from npm install)
    group_id TEXT,                         -- Device group (from subscription)

    -- Funnel stage
    stage TEXT NOT NULL,                   -- 'visit' | 'install' | 'session' | 'subscriber'

    -- Attribution (first touch from landing page)
    utm_source TEXT,
    utm_medium TEXT,
    utm_campaign TEXT,
    promo_code TEXT,
    referrer_domain TEXT,

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_funnel_stage ON funnel_events(stage, created_at);
CREATE INDEX IF NOT EXISTS idx_funnel_visitor ON funnel_events(visitor_id);
CREATE INDEX IF NOT EXISTS idx_funnel_device ON funnel_events(device_id);
CREATE INDEX IF NOT EXISTS idx_funnel_campaign ON funnel_events(utm_campaign, stage);

-- NPM download stats (populated by daily cron job)
CREATE TABLE IF NOT EXISTS npm_downloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    date TEXT NOT NULL UNIQUE,             -- YYYY-MM-DD format
    downloads INTEGER NOT NULL,            -- Daily download count

    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_npm_date ON npm_downloads(date);

-- Email signups from landing page
CREATE TABLE IF NOT EXISTS email_signups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    email TEXT NOT NULL UNIQUE,

    -- Attribution from first visit
    visitor_id TEXT,
    utm_source TEXT,
    utm_medium TEXT,
    utm_campaign TEXT,
    promo_code TEXT,
    referrer_domain TEXT,

    -- Status
    confirmed INTEGER NOT NULL DEFAULT 0,  -- For future double opt-in (currently single opt-in)
    unsubscribed INTEGER NOT NULL DEFAULT 0,

    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    unsubscribed_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_email_signups_created ON email_signups(created_at);
CREATE INDEX IF NOT EXISTS idx_email_signups_campaign ON email_signups(utm_campaign);
