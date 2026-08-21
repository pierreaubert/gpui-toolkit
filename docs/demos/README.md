# Generated WASM demo gallery

The demo catalog is the input to the snapshot and gallery pipeline. Section
inventories come from the Rust showcase applications, so adding a section to
an application automatically adds it to the capture matrix.

Build the static gallery from the repository root:

```bash
just demo-site
```

The output is written to `target/demo-site/`. To capture the live WASM apps as
well, use a WebGPU-capable Chromium and run:

```bash
just wasm-gallery
```

The generated site contains contact sheets, lazy-loaded thumbnails, full-size
images, source metadata, and links to the corresponding live demo routes.
The same command writes `README-snippet.md` with the featured thumbnails and
routes used by the repository README; the committed README points directly at
the published thumbnails so it updates when the Pages site is rebuilt.
The site also emits `_headers` for hosts that support COOP/COEP configuration.
GitHub Pages is suitable for the static snapshot gallery; live WASM requires a
host that preserves the cross-origin-isolated headers required by the web
backend.
