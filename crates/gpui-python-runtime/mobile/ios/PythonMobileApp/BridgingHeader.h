#ifndef BridgingHeader_h
#define BridgingHeader_h

#include <stdbool.h>

// Rust FFI functions exported by the python_mobile_ios staticlib + gpui-ios

// Initialize and run the embedded-Python GPUI app
void python_mobile_ios_start(void);

// Frame rendering — call from CADisplayLink
void gpui_ios_request_current_frame(void);

// Lifecycle
void gpui_ios_will_enter_foreground(void *app_ptr);
void gpui_ios_did_become_active(void *app_ptr);
void gpui_ios_will_resign_active(void *app_ptr);
void gpui_ios_did_enter_background(void *app_ptr);
void gpui_ios_will_terminate(void *app_ptr);

#endif
