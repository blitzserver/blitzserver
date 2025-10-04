-- Populate settings table with environment variables used in the application
INSERT OR IGNORE INTO settings (key, value) VALUES
    ('UPLOAD_ADDRESS', NULL),
    ('ADDRESS', NULL),
    ('GAMES_ADDRESS', NULL),
    ('PORT_RANGE', NULL),
    ('ANTIBOT_QUESTION', NULL),
    ('ANTIBOT_ANSWER', NULL),
    ('MIN_VERSION', NULL),
    ('DATABASE_URL', NULL),
    ('COOKIE_KEY', NULL),
    ('WEB_PORT', NULL);
