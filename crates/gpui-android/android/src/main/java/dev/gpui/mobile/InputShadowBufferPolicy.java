package dev.gpui.mobile;

/** Shared, JVM-testable limits for the IME's non-authoritative shadow buffer. */
final class InputShadowBufferPolicy {
    static final int MAX_CODE_UNITS = 4_096;

    private InputShadowBufferPolicy() {}

    static int trimPrefixLength(int currentLength) {
        return Math.max(0, currentLength - MAX_CODE_UNITS);
    }
}
