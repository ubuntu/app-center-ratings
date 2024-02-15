-- Add up migration script here

CREATE TABLE snap_categories (
    id SERIAL PRIMARY KEY,
    snap_id CHAR(32) NOT NULL,
    category VARCHAR NOT NULL
);
