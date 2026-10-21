# PackageType zero value is SNAP, not UNSPECIFIED

When adding deb support (UD134) we introduced a `PackageType` proto enum with `PACKAGE_TYPE_SNAP = 0`, deliberately breaking the usual `*_UNSPECIFIED = 0` convention. Existing clients (the App Center and the Snap Store) don't send the field, so proto3 decodes it as the zero value; making that zero mean "snap" keeps every existing request meaning exactly what it meant before, with no client changes. The same default (`0 = snap`) is mirrored in the `votes.package_type` column.

## Consequences

The server cannot tell "client forgot to set package type" from "client meant snap". Do not "fix" this by renumbering: doing so would silently reinterpret every existing client's traffic.
