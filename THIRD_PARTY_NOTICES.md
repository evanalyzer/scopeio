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

`cargo deny check licenses` (see `deny.toml`) enforces the allowlist for
all features.
