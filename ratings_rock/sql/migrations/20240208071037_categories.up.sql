-- Add up migration script here

CREATE TABLE snap_categories (
    id SERIAL PRIMARY KEY,
    snap_id CHAR(32) NOT NULL,
    category INTEGER NOT NULL,
    CONSTRAINT category CHECK (category BETWEEN 0 AND 19)
);
