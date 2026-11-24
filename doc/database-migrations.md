# Database names and the initial migration

Migration source files now live in
[`ratings_rock/sql/migrations/`](../ratings_rock/sql/migrations/), relative to the
repository root. SQLx's `sql/migrations` path remains relative to the Rust crate
in `ratings_rock/`; use `cd ratings_rock` for manual Cargo or rock packing commands.
The layout move does not change migration versions, SQL contents, or checksums;
the checksum history and reconciliation procedure below are unchanged.

Schema migrations use the database selected by `APP_POSTGRES_URI`; they do not
create a database or manage its `CONNECT` permission. Juju/DBaaS owns that access.
In PS7 staging the SQL database is **`stg-app-center-rating`**. Do not rename it to
`ratings` or grant the application superuser privileges to run migrations.

The initial migration previously hardcoded an obsolete `ratings` database in a
`GRANT CONNECT`; its down migration revoked that permission. Both statements and
the obsolete manual database-creation comments have been removed. Table, sequence,
and index definitions are unchanged. The local lab's database name can remain
`ratings`; it is not a requirement of the SQL migrations.

## Fresh database

No migration-record adjustment is needed. The relation user needs `CONNECT` and
appropriate schema/DDL permissions supplied by the approved provider, not a
migration targeting a different database. Build a new Ratings image: migrations
are embedded in the binary, so repacking only the charm does not include this fix.

Test initialization and a second startup through PgBouncer with the **Ratings
relation user**, not PgBouncer's privileged backend account. Session pooling must
preserve SQLx's prepared statements and session-level advisory migration locks.

## Existing lab or restored staging database

SQLx validates applied migration checksums. Editing even comments in an applied
migration changes its SHA-384 checksum, so the new image intentionally refuses a
mismatched history instead of silently ignoring it. Adding a later migration
cannot fix a failing initial migration on a fresh database.

For version **`20230829085908`** (initial up migration), the reviewed change is:

| Version of the SQL file | SHA-384 / SQLx checksum (hex) |
| --- | --- |
| Before removal of obsolete database setup | `a037200236f555d3f84b4e6660f577b62d13b1722759e92d6f2425691f6ce2c3579586a3dcd6785e0f84fd8e01d08d6b` |
| After removal | `a4edb898bdd26c184dcf0595a379c4e1557c588b1789e236bfaa900032d5f104d572f0556985da78895a9b45a8079ac2` |

Before a rollout to an existing database:

1. Verify the approved target and inspect its schema and `_sqlx_migrations` history.
   A restored PS6 schema without matching migration records needs a separately
   reviewed baseline; do not mark migrations applied merely because tables exist.
2. Take a recoverable database backup and agree a maintenance window. Ensure no
   Ratings instance, SQLx CLI, or other migrator is running against this database.
3. If the recorded successful initial migration has exactly the old checksum
   above, review and approve reconciliation of **that one metadata row**. The
   schema change is empty; connection privileges stay with Juju/DBaaS. Unknown
   checksums or failed migrations require investigation, not a blanket repair.
4. On an approved staging connection, the guarded transaction below updates only
   the known checksum. A database already using the new checksum is a no-op.
5. Deploy the new image and verify startup/migrations and authenticated gRPC calls.
   Reverting to an old image with the old embedded migration will likewise require
   an explicitly reviewed checksum plan; do not assume image rollback is sufficient.

Inspection, using your approved `psql` connection:

```sql
SELECT current_database();
SELECT version, description, success, encode(checksum, 'hex') AS checksum
FROM _sqlx_migrations
ORDER BY version;
```

**Optional operator-run reconciliation for staging only.** This is a database
metadata mutation, not automatic application behavior. Do not run it on the PS6
export source or use it to hide an unrelated schema/migration mismatch.

```sql
\set ON_ERROR_STOP on
BEGIN;
SET LOCAL lock_timeout = '5s';
DO $$
DECLARE
    recorded_checksum bytea;
    recorded_success boolean;
    old_checksum constant bytea := decode(
        'a037200236f555d3f84b4e6660f577b62d13b1722759e92d6f2425691f6ce2c3579586a3dcd6785e0f84fd8e01d08d6b', 'hex');
    new_checksum constant bytea := decode(
        'a4edb898bdd26c184dcf0595a379c4e1557c588b1789e236bfaa900032d5f104d572f0556985da78895a9b45a8079ac2', 'hex');
BEGIN
    IF current_database() <> 'stg-app-center-rating' THEN
        RAISE EXCEPTION 'Refusing checksum reconciliation outside the staging database';
    END IF;

    LOCK TABLE _sqlx_migrations IN ACCESS EXCLUSIVE MODE;
    SELECT checksum, success INTO recorded_checksum, recorded_success
    FROM _sqlx_migrations WHERE version = 20230829085908;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'No initial migration record; investigate baseline or use fresh-database setup';
    END IF;
    IF recorded_success IS DISTINCT FROM true
       OR recorded_checksum IS NULL
       OR recorded_checksum NOT IN (old_checksum, new_checksum) THEN
        RAISE EXCEPTION 'Unexpected initial migration history; no checksum changed';
    END IF;

    UPDATE _sqlx_migrations SET checksum = new_checksum
    WHERE version = 20230829085908 AND success AND checksum = old_checksum;
END $$;
COMMIT;
```

If it fails, do not continue deployment; roll back the transaction/session and
investigate. For a **confirmed disposable local lab**, either use the team's
approved lab reset procedure (data loss) or approve the analogous narrowly scoped
reconciliation against that local database. The staging guard above deliberately
does not accept the legacy name as evidence that a connection is safe.

No application change disables checksum checks, edits migration records at
startup, or changes the live PS6 source. Only an explicitly approved operator
procedure may reconcile an already-applied checksum.
