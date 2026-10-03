# Browser platform work

Local tracking document. Automatic approval review rejected remote issue creation because it would disclose project details without explicit publication authorization.

## Implemented

- Browser-compatible clocks in charts and MiniApp.
- Embedded licensed IBM Plex Sans and Lilex fonts for browser SVG chart labels.
- Retained orbit and pan gestures across redraws; outside release ends gestures.
- Pointer capture with cancellation, lost-capture, and blur cleanup.
- Plain-text browser clipboard paste and asynchronous clipboard writes.
- Opt-in Control+A selection for Windows/Linux inputs, preserving native Emacs bindings by default.
- External Input values synchronize active edits without discarding uncommitted typing.

Speaker Lab supplies same-origin HTTP, versioned JSON import/export, transactional IndexedDB draft storage, and graphics-device-loss recovery through a small browser adapter. These are application facilities; the toolkit has not gained a general typed platform API for them.

## Verification

Focused Input synchronization and retained-orbit tests pass. Toolkit checks and focused Clippy pass with the existing approximate-constant lint allowed. The pinned application patches compile on WASM; Chromium/SwiftShader browser checks cover continuous orbit, text editing, native solve/reopen, and device-loss draft recovery.

## Remaining toolkit work

- Accessibility DOM bridge for existing ARIA metadata and keyboard focus semantics.
- Typed file/storage/network APIs and reusable device-loss lifecycle support.
- Touch, IME, clipboard permission, Firefox/Safari, and physical WebGPU rendering qualification.
- Investigate retained smooth scalar rendering on software WebGPU; Speaker Lab uses filled contours.

Portable application patches are pinned until this branch can be published and consumed by revision.
