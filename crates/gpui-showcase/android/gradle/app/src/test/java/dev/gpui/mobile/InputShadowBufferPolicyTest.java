package dev.gpui.mobile;

import static org.junit.Assert.assertEquals;

import org.junit.Test;

public final class InputShadowBufferPolicyTest {
    @Test
    public void retains_at_most_the_configured_number_of_utf16_code_units() {
        assertEquals(0, InputShadowBufferPolicy.trimPrefixLength(-1));
        assertEquals(0, InputShadowBufferPolicy.trimPrefixLength(0));
        assertEquals(0, InputShadowBufferPolicy.trimPrefixLength(4_096));
        assertEquals(1, InputShadowBufferPolicy.trimPrefixLength(4_097));
        assertEquals(2_048, InputShadowBufferPolicy.trimPrefixLength(6_144));
    }
}
