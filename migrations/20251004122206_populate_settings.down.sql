-- Remove the settings entries added by the up migration
DELETE FROM settings WHERE key IN (
    'PORT_RANGE',
    'ANTIBOT_QUESTION',
    'ANTIBOT_ANSWER',
    'MIN_VERSION',
    'DATABASE_URL',
    'COOKIE_KEY',
    'WEB_PORT'
);
