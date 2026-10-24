# Third-party notices

scopeio does not contain code ported from other projects. It depends on the
following crates (all transitive dependencies of the default features):

| Crate | Licence | Used for |
|---|---|---|
| flate2 | MIT OR Apache-2.0 | Deflate/zlib streams |
| zlib-rs | Zlib | Deflate backend of flate2 |
| weezl | MIT OR Apache-2.0 | TIFF LZW |
| zune-jpeg, zune-core | MIT OR Apache-2.0 OR Zlib | JPEG |
| ruzstd, twox-hash | MIT | Zstandard |
| roxmltree, memchr | MIT OR Apache-2.0; Unlicense OR MIT | OME-XML |
| serde_json, serde_core, itoa, zmij | MIT OR Apache-2.0; MIT | Zarr metadata |
| lz4_flex | MIT | LZ4 in Blosc and numcodecs |
| hayro-jpeg2000 | Apache-2.0 OR MIT | JPEG 2000 |
| zstd, zstd-sys (optional `zstd-c`) | MIT; BSD-3-Clause (libzstd under its BSD licence) | Zstandard |

`cargo deny check licenses` (see `deny.toml`) enforces the allowlist for
all features.
