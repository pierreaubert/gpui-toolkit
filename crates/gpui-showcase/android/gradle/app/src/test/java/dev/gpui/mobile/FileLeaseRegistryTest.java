package dev.gpui.mobile;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

import java.io.File;
import java.io.IOException;
import java.util.concurrent.atomic.AtomicLong;
import org.junit.Test;

public final class FileLeaseRegistryTest {
    @Test
    public void lease_resolves_until_its_expiry_then_is_removed() throws IOException {
        AtomicLong clock = new AtomicLong(100);
        FileLeaseRegistry registry = new FileLeaseRegistry(clock::get);
        File file = File.createTempFile("gpui-file-provider", ".txt");
        file.deleteOnExit();

        FileLeaseRegistry.Registration registration = registry.register(file);
        assertEquals(file, registry.resolve(registration.token));

        clock.set(registration.expiresAtUptimeMillis);
        assertNull(registry.resolve(registration.token));
        assertNull(registry.resolve(registration.token));
    }

    @Test
    public void scheduled_cleanup_only_removes_its_own_registration() throws IOException {
        AtomicLong clock = new AtomicLong(100);
        FileLeaseRegistry registry = new FileLeaseRegistry(clock::get);
        File file = File.createTempFile("gpui-file-provider", ".txt");
        file.deleteOnExit();

        FileLeaseRegistry.Registration registration = registry.register(file);
        assertTrue(registry.remove(registration));
        assertFalse(registry.remove(registration));
        assertNull(registry.resolve(registration.token));
    }
}
