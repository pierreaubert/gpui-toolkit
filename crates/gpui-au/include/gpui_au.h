#ifndef GPUI_AU_H
#define GPUI_AU_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AuContext AuContext;

AuContext *gpui_au_create(
    void *ns_view,
    float width,
    float height,
    float scale,
    const char *plugin_type
);
void gpui_au_destroy(AuContext *context);
void gpui_au_request_frame(AuContext *context);
void gpui_au_resize(AuContext *context, float width, float height, float scale);
void gpui_au_set_active(AuContext *context, bool is_active);
void gpui_au_set_hovered(AuContext *context, bool is_hovered);

void gpui_au_mouse_down(
    AuContext *context,
    float x,
    float y,
    int32_t button,
    int32_t click_count,
    uint32_t modifier_flags
);
void gpui_au_mouse_up(
    AuContext *context, float x, float y, int32_t button, uint32_t modifier_flags
);
void gpui_au_mouse_moved(AuContext *context, float x, float y, uint32_t modifier_flags);
void gpui_au_mouse_dragged(
    AuContext *context, float x, float y, int32_t button, uint32_t modifier_flags
);
void gpui_au_scroll_wheel(
    AuContext *context,
    float x,
    float y,
    float delta_x,
    float delta_y,
    uint32_t modifier_flags
);

void gpui_au_key_down(
    AuContext *context,
    uint16_t key_code,
    const char *characters,
    const char *characters_ignoring_modifiers,
    uint32_t modifier_flags,
    bool is_repeat
);
void gpui_au_key_up(
    AuContext *context,
    uint16_t key_code,
    const char *characters,
    const char *characters_ignoring_modifiers,
    uint32_t modifier_flags
);

/* Minimal parameter tree (AUParameterTree analogue). */
size_t gpui_au_parameter_count(AuContext *context);
float gpui_au_parameter_value(AuContext *context, uint32_t id, bool *ok);
bool gpui_au_parameter_set(AuContext *context, uint32_t id, float value);
bool gpui_au_parameter_register(
    AuContext *context,
    uint32_t id,
    float min_value,
    float max_value,
    float default_value,
    const char *name
);

/* Realtime health counters. Each out-pointer is optional; a null context
   leaves all outputs untouched. */
void gpui_au_frame_stats(AuContext *context, size_t *dropped, size_t *coalesced);

/* Plugin state persistence (fullState analogue). See ffi.rs for the
   null-buffer size-query contract of gpui_au_save_state. */
size_t gpui_au_save_state(AuContext *context, uint8_t *out, size_t capacity, size_t *written);
bool gpui_au_load_state(AuContext *context, const uint8_t *data, size_t len);

void gpui_au_insert_text(AuContext *context, const char *text);
void gpui_au_set_marked_text(
    AuContext *context,
    const char *text,
    size_t selected_location,
    size_t selected_length
);
void gpui_au_unmark_text(AuContext *context);
void gpui_au_delete_backward(AuContext *context);

#ifdef __cplusplus
}
#endif

#endif
