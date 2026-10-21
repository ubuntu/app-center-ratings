-- WARNING: this down migration DELETES DATA. The pre-UD134 schema can only
-- hold snap votes with a 32 character Snap ID, so it deletes:
--   * every deb vote
--   * any snap vote whose App ID is longer than 32 characters
DELETE FROM votes WHERE package_type <> 0;
DELETE FROM votes WHERE LENGTH(app_id) > 32;

DROP INDEX idx_votes_package_type_app_id;
DROP INDEX idx_votes_unique_user_deb_version;
DROP INDEX idx_votes_unique_user_snap_revision;

ALTER TABLE votes DROP CONSTRAINT votes_package_shape;
ALTER TABLE votes
    DROP COLUMN package_version,
    DROP COLUMN package_type,
    ALTER COLUMN snap_revision SET NOT NULL;
ALTER TABLE votes ADD CONSTRAINT votes_snap_revision_check CHECK (snap_revision > 0);

ALTER TABLE votes ALTER COLUMN app_id TYPE CHAR(32);
ALTER TABLE votes RENAME COLUMN app_id TO snap_id;

CREATE UNIQUE INDEX idx_votes_unique_user_snap ON votes (user_id_fk, snap_id, snap_revision);

DROP TABLE package_types;
