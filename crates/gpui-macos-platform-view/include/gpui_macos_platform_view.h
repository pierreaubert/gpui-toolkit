/* gpui-macos-platform-view — umbrella header (hand-maintained; regenerate
 * with cbindgen using cbindgen.toml in this crate when the API changes). */

#ifndef GPUI_MACOS_PLATFORM_VIEW_H
#define GPUI_MACOS_PLATFORM_VIEW_H

#ifdef __cplusplus
extern "C" {
#endif

/// JSON snapshot of every live platform view frame, in GPUI points with a
/// top-left origin: `[{"id": "...", "x": 0, "y": 0, "width": 0,
/// "height": 0, "z_index": 0, "visible": true, "scale_factor": 2.0}]`.
///
/// The caller owns the returned string and must release it with
/// `gpui_macos_platform_views_free_string`. Returns NULL only on
/// allocation/serialization failure.
char *gpui_macos_platform_views_snapshot_json(void);

/// Free a string returned by `gpui_macos_platform_views_snapshot_json`.
void gpui_macos_platform_views_free_string(char *ptr);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // GPUI_MACOS_PLATFORM_VIEW_H
