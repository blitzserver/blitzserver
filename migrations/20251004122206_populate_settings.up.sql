-- Populate settings table with environment variables used in the application
-- COOKIE_KEY is left NULL and should be generated on first run
INSERT OR IGNORE INTO settings (key, value) VALUES
    ('UPLOAD_ADDRESS', 'http://localhost:8080'),
    ('ADDRESS', 'http://localhost:8080'),
    ('PORT_RANGE', '10000,10999'),
    ('ANTIBOT_QUESTION', NULL),
    ('ANTIBOT_ANSWER', NULL),
    ('MIN_VERSION', '554'),
    ('COOKIE_KEY', NULL),
    ('WEB_PORT', '8080'),
    ('ADMIN_PASSWORD', NULL);
