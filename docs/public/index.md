# ComposeLens

ComposeLens is the Compose document library behind BoxFerry. Use it directly when a Rust program needs to inspect, validate, edit, merge, or render Compose without hiding when interpolation and other processing occur.

## What the library owns

- loss-aware YAML parsing and source locations;
- typed Compose documents and merged project views;
- caller-controlled interpolation, includes, profiles, and path resolution;
- compatibility findings for explicitly selected implementations and versions; and
- preservation edits plus deterministic Compose rendering.

ComposeLens does not start containers, contact a container runtime, read process environment variables implicitly, or convert Compose into another format. Cross-format conversion belongs to [BoxFerry](https://boxferry.dev/docs/).

## Choose a topic

- [Model](model/) explains the document and project views.
- [Parsing and rendering](parsing-rendering/) shows the explicit processing pipeline.
- [Diagnostics](diagnostics/) covers codes, labels, source spans, and partial results.
- [Compatibility](compatibility/) separates syntax support from implementation evidence.
- [Rust API](https://boxferry.dev/docs/api/compose-lens/) lists every public item.

Add the latest compatible release with `cargo add compose-lens`. Rust 1.85.0 or newer is required.
