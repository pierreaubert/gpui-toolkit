package dev.gpui.mobile;

import static org.junit.Assert.assertEquals;

import org.junit.Test;

public class GpuiAccessibilityNodeIdTest {
    @Test
    public void decodesUnsignedIdsAcrossTheSignedLongBoundary() throws Exception {
        long highBitId = GpuiAccessibilityNodeId.fromHex("8000000000000001");
        long maximumId = GpuiAccessibilityNodeId.fromHex("ffffffffffffffff");

        assertEquals(Long.MIN_VALUE + 1, highBitId);
        assertEquals("9223372036854775809", Long.toUnsignedString(highBitId));
        assertEquals("18446744073709551615", Long.toUnsignedString(maximumId));
        assertEquals(highBitId, GpuiAccessibilityNodeId.fromJsonValue("8000000000000001"));
    }

    @Test(expected = NumberFormatException.class)
    public void rejectsIdsThatAreNotFixedWidthHexStrings() {
        GpuiAccessibilityNodeId.fromHex("100");
    }
}
