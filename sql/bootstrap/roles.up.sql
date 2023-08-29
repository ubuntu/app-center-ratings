CREATE USER migration_user WITH PASSWORD 'strongpassword';
CREATE USER service WITH PASSWORD 'covfefe!1';
CREATE DATABASE ratings;
-- /c ratings;
GRANT CONNECT ON DATABASE ratings TO migration_user;
GRANT USAGE, CREATE ON SCHEMA public TO migration_user;
