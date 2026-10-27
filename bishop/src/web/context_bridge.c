#include <emscripten/emscripten.h>
#include <emscripten/html5.h>

int bishop_open_webgl2(const char *selector) {
    EmscriptenWebGLContextAttributes attrs;
    emscripten_webgl_init_context_attributes(&attrs);
    attrs.majorVersion = 2;
    EMSCRIPTEN_WEBGL_CONTEXT_HANDLE context = emscripten_webgl_create_context(selector, &attrs);
    if (context <= 0) return (int)context;
    if (emscripten_webgl_make_context_current(context) != EMSCRIPTEN_RESULT_SUCCESS) {
        emscripten_webgl_destroy_context(context);
        return -1;
    }
    return (int)context;
}

int bishop_make_webgl2_current(int context) {
    return (int)emscripten_webgl_make_context_current(context);
}

void bishop_close_webgl2(int context) {
    emscripten_webgl_destroy_context(context);
}

int bishop_canvas_size(const char *selector, int *width, int *height) {
    return (int)emscripten_get_canvas_element_size(selector, width, height);
}

double bishop_now_ms(void) {
    return emscripten_get_now();
}

int bishop_set_canvas_cursor(const char *selector, const char *css_cursor) {
    return EM_ASM_INT({
        const canvas = document.querySelector(UTF8ToString($0));
        if (!canvas) return -1;
        canvas.style.cursor = UTF8ToString($1);
        return 0;
    }, selector, css_cursor);
}

double bishop_canvas_scale(const char *selector) {
    return EM_ASM_DOUBLE({
        const canvas = document.querySelector(UTF8ToString($0));
        if (!canvas) return 0;
        const bounds = canvas.getBoundingClientRect();
        return bounds.width > 0 ? canvas.width / bounds.width : 0;
    }, selector);
}

int bishop_fullscreen_status(void) {
    EmscriptenFullscreenChangeEvent status;
    if (emscripten_get_fullscreen_status(&status) != EMSCRIPTEN_RESULT_SUCCESS) return -1;
    return status.isFullscreen ? 1 : 0;
}

int bishop_toggle_fullscreen(const char *selector) {
    int active = bishop_fullscreen_status();
    if (active < 0) return -1;
    EMSCRIPTEN_RESULT result = active
        ? emscripten_exit_fullscreen()
        : emscripten_request_fullscreen(selector, EM_FALSE);
    if (result != EMSCRIPTEN_RESULT_SUCCESS) return -1;
    return active ? 0 : 1;
}
