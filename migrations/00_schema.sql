CREATE TABLE IF NOT EXISTS entries (
    id integer PRIMARY KEY NOT NULL,
    expression text NOT NULL,
    reading text,
    source text NOT NULL,
    speaker text,
    display text,
    file text NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reading ON entries(reading);
CREATE INDEX IF NOT EXISTS idx_speaker ON entries(speaker);
CREATE INDEX IF NOT EXISTS idx_expr_reading ON entries(expression, reading);
CREATE INDEX IF NOT EXISTS idx_reading_speaker ON entries(expression, reading, speaker);
CREATE INDEX IF NOT EXISTS idx_all ON entries(expression, reading, source);

CREATE TABLE pitch_accents (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        expression TEXT NOT NULL,
        reading TEXT NOT NULL,
        pitch TEXT NOT NULL,
        count INTEGER NOT NULL DEFAULT 1,
        UNIQUE(expression, reading, pitch)
    );
CREATE INDEX idx_pitch_expression_pitch ON pitch_accents(expression);
CREATE INDEX idx_pitch_reading_pitch ON pitch_accents(reading);
CREATE INDEX idx_pitch_expression_reading_pitch ON pitch_accents(expression, reading);
