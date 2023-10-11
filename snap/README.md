# App Center Ratings Snap

This directory contains files used to build the App Center Ratings snap.

## Installation 

The snap provides a wrapper used to collect environment variables via `snapctl` in order for them to be configurable post the snap being built.

You can install the App Center Ratings snap manually like so:

```bash
# Install the snap 
$ sudo snap install ratings --channel stable 

# Update an environment variable
sudo snap set ratings app-log-level=debug

# Restart the service to reload environment variables into the service
sudo snap restart ratings
```

## Configuration

This Snap is intended to run within a charm where certain configuration settings are unknown until the charm installs the snap and establishes relations to a database.

The list of config options are:

| Name                        | Options                          | Default                                                        | Description             |
|:----------------------------|:--------------------------------|:---------------------------------------------------------------|:------------------------|
| `app-jwt-secret`            | Any string                      | `randomized`                                                     | JWT secret              |
| `app-log-level`             | `error`, `warn`, `info`, `debug` | `info`                                                         | Log level               |
| `app-postgres-uri`          | Any string                      | `` | Service connection     |
| `app-migration-postgres-uri`| Any string                      | `` | Migrator connection    |

Config options can be set using: `sudo snap set ratings <option>=<value>`
