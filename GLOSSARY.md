# App Center Ratings

The backend that collects community votes on apps installed through the Ubuntu App Center and turns them into ratings.

## Language

### Identity

**App**:
Something users can rate, identified by the pair of its Package Type and App ID.
_Avoid_: Snap (when the package type is not known), package, component

**Package Type**:
The packaging format an App is distributed in: snap or deb.
_Avoid_: Kind, source

**App ID**:
The globally unique, stable identifier a vote is cast against. For a snap it is currently the Snap ID; for a deb it is the AppStream Component ID.
_Avoid_: Snap ID (when the package type is not known), package name

**Snap ID**:
The opaque 32-character identifier the Snap Store assigns to a snap. A snap's App ID today, adopted by mistake; the Snap Name is the intended identity.

**Snap Name**:
The globally unique, permanent name of a snap in the Snap Store (e.g. `firefox`). The intended future App ID for snaps; not yet used as one.

**AppStream Component ID**:
The reverse-DNS identifier (e.g. `org.gnome.Calculator`) from a deb's AppStream metadata. Debs without one cannot be rated.
_Avoid_: Package name, desktop ID

**Package Version**:
The specific release of an App that a vote is cast against. For a deb it is the upstream version from the AppStream metadata; debs without one cannot be rated.
_Avoid_: Debian package version

**Snap Revision**:
The Snap Store's integer revision for a snap release.

### Ratings

**Vote**:
A single user's thumbs up or thumbs down on one Package Version of an App.

**Rating**:
The aggregate of all Votes for an App, expressed as a Ratings Band.

**Ratings Band**:
The coarse bucket (very good to very poor, or insufficient votes) a Rating falls into, derived from the Wilson score of its Votes.
