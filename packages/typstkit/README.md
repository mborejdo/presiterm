# typstkit

In-process [Typst](https://typst.app) through [typst-as-lib](https://crates.io/crates/typst-as-lib):
compile a source string, or a file under a root folder, to PNG, SVG or PDF. The default fonts are
the faces [swatchbook](../swatchbook) vendors (Inter, Fira Code); more can be added. With the
`packages` feature (default) `@preview/...` imports are downloaded from packages.typst.org once and
cached where the typst CLI caches them.

```rust
let engine = typstkit::Engine::builder().build();
let png = engine.png(typstkit::Main::Source("#set page(width: auto, height: auto)\nHello"), 144.0)?;
# Ok::<(), anyhow::Error>(())
```

License: MIT OR Apache-2.0.
