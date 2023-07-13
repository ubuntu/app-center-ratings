# Provision the db

```psql
CREATE DATABASE ratings;

CREATE USER service WITH PASSWORD 'covfefe!1';

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    user_id CHAR(64) NOT NULL UNIQUE,
    created TIMESTAMP NOT NULL,
    last_seen TIMESTAMP NOT NULL
);

GRANT ALL PRIVILEGES ON TABLE users TO service;
GRANT CONNECT ON DATABASE ratings TO service;
```
