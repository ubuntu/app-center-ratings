-- Create database and then execute the second set of commands when connected
-- to the ratings database
--

-- Stage 1

-- CREATE DATABASE IF EXISTS ratings;

-- Stage 2
--

CREATE TABLE users (
   id SERIAL PRIMARY KEY,
   client_hash CHAR(64) NOT NULL UNIQUE, -- sha256([$user:$machineId])
   created TIMESTAMPTZ NOT NULL DEFAULT NOW(),
   last_seen TIMESTAMPTZ NOT NULL
);

CREATE TABLE votes (
   id SERIAL PRIMARY KEY,
   created TIMESTAMPTZ NOT NULL DEFAULT NOW(),
   user_id_fk INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
   snap_id CHAR(32) NOT NULL,
   snap_revision INT NOT NULL CHECK (snap_revision > 0),
   vote_up BOOLEAN NOT NULL
);

-- Create a unique index on user_id, snap_id, and snap_revision.
-- This ensures that the combination of user_id, snap_id, and snap_revision
-- is unique in the votes table. It helps enforce the rule that a user
-- can't vote more than once for the same snap revision.
CREATE UNIQUE INDEX idx_votes_unique_user_snap ON votes (user_id_fk, snap_id, snap_revision);

-- Grant privileges to the user currently running the script
GRANT ALL PRIVILEGES ON TABLE users TO CURRENT_USER;
GRANT USAGE, SELECT ON SEQUENCE users_id_seq TO CURRENT_USER;
GRANT ALL PRIVILEGES ON TABLE votes TO CURRENT_USER;
GRANT USAGE, SELECT ON SEQUENCE votes_id_seq TO CURRENT_USER;
GRANT CONNECT ON DATABASE ratings TO CURRENT_USER;
