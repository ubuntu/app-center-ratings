# Provision the db

```psql
CREATE USER service WITH PASSWORD 'covfefe!1';

\c ratings

CREATE TABLE users (
    token CHAR(64) NOT NULL
);

GRANT ALL PRIVILEGES ON TABLE users TO service;

GRANT CONNECT ON DATABASE ratings TO service;

```
