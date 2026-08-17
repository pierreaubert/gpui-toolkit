import unittest

from gpui_toolkit.resources import ResourceError, ResourceKind, ResourceStore, StaleResourceError


class ResourceStoreTests(unittest.TestCase):
    def test_generation_checksum_and_stale_handles(self):
        store = ResourceStore(16)
        first = store.put("series", b"one", kind=ResourceKind.CHART_SERIES, shape=(3,))
        self.assertEqual(store.read(first), b"one")
        replacement = store.put("series", b"two", kind=ResourceKind.CHART_SERIES)
        self.assertEqual(replacement.generation, 2)
        with self.assertRaises(StaleResourceError):
            store.read(first)
        self.assertEqual(store.read(replacement), b"two")

    def test_replacement_keeps_a_retained_previous_generation_alive(self):
        store = ResourceStore(16)
        first = store.put("mesh", b"first", kind=ResourceKind.MESH)
        store.retain(first)

        second = store.put("mesh", b"second", kind=ResourceKind.MESH)

        self.assertEqual(store.read(first), b"first")
        self.assertEqual(store.read(second), b"second")
        self.assertEqual(store.stats().referenced_entries, 1)
        self.assertEqual(store.stats().references, 1)
        with self.assertRaises(ResourceError):
            store.drop(first)

        store.release(first)
        with self.assertRaises(StaleResourceError):
            store.read(first)
        self.assertEqual(store.read(second), b"second")

    def test_failed_replacement_does_not_discard_the_previous_generation(self):
        store = ResourceStore(8)
        first = store.put("mesh", b"1234", kind=ResourceKind.MESH)
        retained = store.put("other", b"5678", kind=ResourceKind.MESH)
        store.retain(retained)

        with self.assertRaises(ResourceError):
            store.put("mesh", b"abcdefgh", kind=ResourceKind.MESH)

        self.assertEqual(store.read(first), b"1234")

    def test_budget_and_retention_are_observable(self):
        store = ResourceStore(6)
        retained = store.put("mesh", b"1234", kind=ResourceKind.MESH)
        store.retain(retained)
        with self.assertRaises(ResourceError):
            store.put("other", b"5678")
        store.release(retained)
        other = store.put("other", b"56")
        self.assertEqual(store.read(other), b"56")
        self.assertGreaterEqual(store.stats().evictions, 0)

    def test_explicit_drop_and_clear_preserve_stale_generation_ordering(self):
        store = ResourceStore(32)
        first = store.put("mesh", b"first", kind=ResourceKind.MESH)
        store.drop(first)
        with self.assertRaises(StaleResourceError):
            store.read(first)

        second = store.put("mesh", b"second", kind=ResourceKind.MESH)
        self.assertEqual(second.generation, 2)
        store.clear()
        with self.assertRaises(StaleResourceError):
            store.read(second)

        third = store.put("mesh", b"third", kind=ResourceKind.MESH)
        self.assertEqual(third.generation, 3)

    def test_drop_rejects_retained_resource(self):
        store = ResourceStore(32)
        resource = store.put("mesh", b"mesh", kind=ResourceKind.MESH)
        store.retain(resource)
        with self.assertRaises(ResourceError):
            store.drop(resource)
        store.release(resource)
        store.drop(resource)

    def test_alternating_field_updates_remain_bounded_while_geometry_is_retained(self):
        store = ResourceStore(64)
        geometry = store.put("geometry", b"geometry", kind=ResourceKind.MESH)
        store.retain(geometry)

        maximum_entries = 0
        maximum_bytes = 0
        latest = None
        for generation in range(1, 1001):
            latest = store.put(
                "field",
                bytes([generation % 251]) * 8,
                kind=ResourceKind.MESH,
            )
            stats = store.stats()
            maximum_entries = max(maximum_entries, stats.entries)
            maximum_bytes = max(maximum_bytes, stats.bytes_used)
            self.assertEqual(latest.generation, generation)
            self.assertLessEqual(stats.entries, 2)
            self.assertLessEqual(stats.bytes_used, 64)
            self.assertEqual(store.read(geometry), b"geometry")

        self.assertEqual(maximum_entries, 2)
        self.assertLessEqual(maximum_bytes, 64)
        self.assertIsNotNone(latest)
        self.assertEqual(store.read(latest), bytes([1000 % 251]) * 8)
        store.release(geometry)
        store.clear()
        self.assertEqual(store.stats().entries, 0)
        self.assertEqual(store.stats().bytes_used, 0)


if __name__ == "__main__":
    unittest.main()
