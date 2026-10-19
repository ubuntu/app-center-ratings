-- Generalise votes from snaps to Apps: an App is (package_type, app_id).
-- Existing rows become snap votes (package_type defaults to 0 = snap, see
-- docs/adr/0001-package-type-defaults-to-snap.md).

CREATE TABLE package_types (
    id SMALLINT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

INSERT INTO package_types (id, name) VALUES (0, 'snap'), (1, 'deb');

ALTER TABLE votes RENAME COLUMN snap_id TO app_id;
ALTER TABLE votes ALTER COLUMN app_id TYPE VARCHAR(255);

ALTER TABLE votes
    ADD COLUMN package_type SMALLINT NOT NULL DEFAULT 0 REFERENCES package_types(id),
    ADD COLUMN package_version TEXT NULL,
    ALTER COLUMN snap_revision DROP NOT NULL;

-- Snap votes are keyed by Snap Revision, deb votes by Package Version.
ALTER TABLE votes DROP CONSTRAINT votes_snap_revision_check;
ALTER TABLE votes ADD CONSTRAINT votes_package_shape CHECK (
    (package_type = 0 AND snap_revision IS NOT NULL AND snap_revision > 0 AND package_version IS NULL)
    OR
    (package_type = 1 AND package_version IS NOT NULL AND snap_revision IS NULL)
);

-- A user can vote at most once per snap revision / deb version.
DROP INDEX idx_votes_unique_user_snap;
CREATE UNIQUE INDEX idx_votes_unique_user_snap_revision
    ON votes (user_id_fk, app_id, snap_revision) WHERE package_type = 0;
CREATE UNIQUE INDEX idx_votes_unique_user_deb_version
    ON votes (user_id_fk, app_id, package_version) WHERE package_type = 1;

CREATE INDEX idx_votes_package_type_app_id ON votes (package_type, app_id);
