# Contributing to scopeio

Contributions are welcome. Please read the two points below before opening a
pull request.

## Contributor License Agreement

scopeio is licensed under AGPL-3.0-only with the EVAnalyzer exception and is
also offered under commercial licenses. Every contributor therefore has to
accept the [Contributor License Agreement](CLA.md) once. You keep the
copyright in your work; the CLA lets the maintainer license it under other
terms too, and guarantees that it stays available as open source.

To accept, write this comment on your first pull request:

> I have read the scopeio CLA (version 1.0) and I agree to it.

Pull requests without this comment cannot be merged.

## Clean-room rules

scopeio is a clean-room implementation. Do not open, copy, translate, or work
from the source code of GPL, LGPL, or AGPL readers (for example Bio-Formats,
bioformats-rs, OpenSlide, readlif) or vendor SDKs that forbid reverse
engineering. Allowed sources are format specifications, vendor documentation,
permissively licensed code (with its notice kept in `THIRD_PARTY_NOTICES.md`),
and your own analysis of sample files. Record every source you used in
`PROVENANCE.md`, and start each new source file with the license header and,
for format modules, a provenance header.

New dependencies must use one of the licenses allowed in `deny.toml`.
