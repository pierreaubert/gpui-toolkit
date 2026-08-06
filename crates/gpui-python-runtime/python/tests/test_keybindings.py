import unittest

from gpui_toolkit.keybindings import KeyBinding, KeybindingRegistry, KeymapPreset


class KeybindingRegistryTests(unittest.TestCase):
    def test_registry_discovers_and_serializes_bindings(self):
        registry = KeybindingRegistry()
        run = KeyBinding("simulation.run", "cmd-r", "Run simulation", "Simulation")
        registry.register(run)
        self.assertEqual(registry.search("simulation"), (run,))
        self.assertEqual(registry.to_spec()[0]["command_id"], "simulation.run")

    def test_conflicts_are_grouped_per_preset(self):
        registry = KeybindingRegistry()
        registry.register(KeyBinding("one", "cmd-r", "One"))
        registry.register(KeyBinding("two", "CMD-R", "Two"))
        registry.register(KeyBinding("vim", "r", "Vim"), KeymapPreset.VIM)
        conflict = registry.conflicts()[0]
        self.assertEqual(conflict.key, "cmd-r")
        self.assertEqual(len(conflict.bindings), 2)
        self.assertEqual(registry.conflicts(KeymapPreset.VIM), ())


if __name__ == "__main__":
    unittest.main()
