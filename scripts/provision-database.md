# Provision the db

```psql
CREATE USER service WITH PASSWORD 'covfefe!1';

\c ratings

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    instance_id CHAR(64) NOT NULL UNIQUE,
    created TIMESTAMP NOT NULL,
    last_seen TIMESTAMP NOT NULL
);

GRANT ALL PRIVILEGES ON TABLE users TO service;

GRANT CONNECT ON DATABASE ratings TO service;
```
