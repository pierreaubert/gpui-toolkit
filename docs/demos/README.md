# Browser landing page and demo menus

`just demo-site` builds the five live WASM routes and generates the landing
page from `catalog.json` and the Rust showcase manifests. The landing template,
styles, and interaction code live in `landing.html`, `landing.css`, and
`landing.js`. Section inventories come from the Rust applications, so new
sections automatically enter the capture matrix. `just wasm-gallery` captures
all live previews with a WebGPU-capable Chromium.

Output lives in `target/demo-site/`. The complete catalog includes lazy-loaded
thumbnails, live routes, source links, and one featured example across every
theme and design style. The generator also writes `README-snippet.md`; the
repository README points to the published thumbnail URLs.

Publish the generated directory manually to https://gpui-toolkit.spinorama.org/
using `just wasm-upload`. The generated `_headers` declares COOP/COEP, which the
host must preserve for threaded WASM.

The full D3rs Spinorama viewer uses the live speaker catalog and measurements
from `api.spinorama.org`, with eight views, curve selection, zoom, polar
frequencies, contour modes, and interactive 3D surfaces/spheres.
The measurement API is key-protected: on first run a popup asks for an API
key (saved in the OS credential store on native builds, memory-only on
wasm, sent as `Authorization: Bearer …`). Without a key the viewer shows
only the public result picture (`?measurement_format=png`) with no access
to measurement data; a rejected (401/403) key reopens the popup.
Three additional sections (`All Graphs (Frequency/Polar/Bars)`) expose
every other spinorama.org graph — reflections, directivity, group delay,
headphone graphs, radar/spider, histograms, and the `filters_*` family —
through generic Plotly trace renderers, and the batch image generator
classifies those payload stems the same way. The upstream
does not send CORS headers. Include `spinorama-nginx.conf` inside the site's
nginx server block before publishing: browser requests use
`/api/spinorama/v1/`. Trunk's Spinorama configuration provides the same proxy
for `just wasm-serve-spinorama`. To preview the assembled site, run
`python3 scripts/serve_demo_site.py` and open `http://127.0.0.1:8090/`.

For the current host, the installer `scripts/install_demo_nginx_proxy.sh`
adds the snippet to `/etc/nginx/sites-available/gpui-toolkit-prod`, validates
nginx, and reloads it; failed validation restores the original configuration.
Stage the installer and `spinorama-nginx.conf` together and run the installer
with sudo. The snippet also enables WASM gzip responses and COOP/COEP headers.
`just wasm-upload` checks the required API route before publishing.

All five Trunk Rust links explicitly select `data-wasm-opt="2"`. Site builds,
development servers, and visual QA servers use `--release`: Cargo applies its
release optimization and thin LTO, wasm-bindgen removes debug information,
and Binaryen optimizes the generated WASM. `wasm-check` remains a compile check.

The viewer starts with KEF R3 measurements. Gallery readiness waits for the
measurement data, rather than just the first canvas frame. Real CEA2034 and
contour response fixtures cover typed arrays and grid validation in Rust tests.

Run `uv run --with pillow --with playwright python scripts/qa_spinorama_browser.py`
against the local assembled site to check all eight views with real recorded
measurements, ensure meshes remain visible after rotation, and change sphere
frequency. Add `--live-api` to verify upstream data loading. This check also
runs in the demo-gallery workflow.

`demo-shell.js` is copied by Trunk into each demo route. It exposes demo,
theme, and design selectors above the GPUI canvas on desktop and mobile.
Appearance changes restart the app using its existing query parameters.
`qa=1` hides this browser chrome for native parity captures, preserving the
native application's viewport. Ordinary gallery captures include the menu.

## Chess regression

The original chess board relied on system fallback for U+2654–U+265F. Neither
IBM Plex Sans nor Lilex includes these glyphs. WASM renders missing-glyph boxes;
the native headless harness intentionally disables system fonts and uses the
same embedded set, so native and WASM could agree on the broken board. The
whole-page tolerant comparison additionally dilutes small missing details.

MiniApp now embeds and registers DejaVu Sans, and chess nodes select that family
explicitly. `qa_chess_content.py` independently requires all 32 pieces and five
distinct back-rank silhouettes on each image. It rejects blank pieces and tofu
boxes even if both images are identical. Native/WASM comparison, renderer
parity, and gallery capture run this assertion. Its tests include the identical-broken-board case,
missing ink, and valid pieces at several scales.

Run the checks with:

```sh
PYTHONPATH=scripts uv run --with pillow --with playwright python -m unittest \
  discover -s scripts/tests -p 'test_qa_chess_content.py'
just wasm-native-parity 8081 chess-desktop
```

To compare a built subroute instead of a Trunk root, pass
`--url http://127.0.0.1:8090/showcase/` to `qa_showcase_native_wasm.py`.
Static hosting must retain COOP/COEP headers for the threaded WASM apps.
