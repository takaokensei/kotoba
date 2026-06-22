CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO settings (key, value) VALUES ('audio_persisted', 'false');
INSERT INTO settings (key, value) VALUES ('practice_language', 'ja');
