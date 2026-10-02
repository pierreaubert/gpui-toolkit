package dev.gpui.mobile;

import org.json.JSONException;

/** Lossless decoder for unsigned AccessKit IDs crossing Android JSON and JNI. */
final class GpuiAccessibilityNodeId {
    private GpuiAccessibilityNodeId() {}

    static long fromJsonValue(Object value) throws JSONException {
        if (value instanceof String) {
            try {
                return fromHex((String) value);
            } catch (NumberFormatException error) {
                throw new JSONException("invalid AccessKit node ID");
            }
        }
        if (value instanceof Number) {
            try {
                return Long.parseUnsignedLong(value.toString(), 10);
            } catch (NumberFormatException error) {
                throw new JSONException("invalid numeric AccessKit node ID");
            }
        }
        throw new JSONException("AccessKit node ID must be a hexadecimal string");
    }

    static long fromHex(String value) {
        if (value == null || value.length() != 16) {
            throw new NumberFormatException("AccessKit node IDs use 16 hexadecimal digits");
        }
        return Long.parseUnsignedLong(value, 16);
    }
}
