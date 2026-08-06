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


if __name__ == "__main__":
    unittest.main()
