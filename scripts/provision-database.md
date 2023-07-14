# Provision the db

```psql
CREATE DATABASE ratings;

CREATE USER service WITH PASSWORD 'covfefe!1';

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    user_id CHAR(64) NOT NULL UNIQUE,
    created TIMESTAMP NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMP NOT NULL
);

CREATE TABLE votes (
    id SERIAL PRIMARY KEY,
    created TIMESTAMP NOT NULL DEFAULT NOW(),
    user_id INT NOT NULL REFERENCES users(id),
    snap_id CHAR(32) NOT NULL,
    snap_revision INT NOT NULL CHECK (snap_revision > 0),
    vote_up BOOLEAN NOT NULL
);

GRANT ALL PRIVILEGES ON TABLE users TO service;
GRANT USAGE, SELECT ON SEQUENCE users_id_seq TO service;

GRANT ALL PRIVILEGES ON TABLE votes TO service;
GRANT USAGE, SELECT ON SEQUENCE votes_id_seq TO service;

GRANT CONNECT ON DATABASE ratings TO service;
```
