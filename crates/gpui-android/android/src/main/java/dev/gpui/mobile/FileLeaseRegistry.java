package dev.gpui.mobile;

import java.io.File;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.LongSupplier;

/** Package-private, JVM-testable storage for short-lived FileProvider grants. */
final class FileLeaseRegistry {
    static final long LEASE_MILLIS = 10 * 60 * 1_000L;

    static final class Registration {
        final String token;
        final long expiresAtUptimeMillis;
        private final FileLease lease;

        Registration(String token, FileLease lease) {
            this.token = token;
            this.expiresAtUptimeMillis = lease.expiresAtUptimeMillis;
            this.lease = lease;
        }
    }

    private static final class FileLease {
        final File file;
        final long expiresAtUptimeMillis;

        FileLease(File file, long expiresAtUptimeMillis) {
            this.file = file;
            this.expiresAtUptimeMillis = expiresAtUptimeMillis;
        }
    }

    private final ConcurrentHashMap<String, FileLease> files = new ConcurrentHashMap<>();
    private final LongSupplier uptimeMillis;

    FileLeaseRegistry(LongSupplier uptimeMillis) {
        this.uptimeMillis = uptimeMillis;
    }

    Registration register(File file) {
        String token = UUID.randomUUID().toString();
        FileLease lease = new FileLease(file, uptimeMillis.getAsLong() + LEASE_MILLIS);
        files.put(token, lease);
        return new Registration(token, lease);
    }

    File resolve(String token) {
        if (token == null) {
            return null;
        }
        FileLease lease = files.get(token);
        if (lease == null) {
            return null;
        }
        if (lease.expiresAtUptimeMillis <= uptimeMillis.getAsLong()) {
            files.remove(token, lease);
            return null;
        }
        return lease.file;
    }

    boolean remove(String token) {
        return token != null && files.remove(token) != null;
    }

    boolean remove(Registration registration) {
        return files.remove(registration.token, registration.lease);
    }
}
