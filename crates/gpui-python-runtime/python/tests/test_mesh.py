import unittest

from gpui_toolkit import mesh


def _rotation_dataset():
    scalars = []
    vectors = []
    for k in range(5):
        for j in range(5):
            for i in range(5):
                x, y = i * 0.25, j * 0.25
                scalars.append(x + y + k * 0.25)
                vectors.append([-y, x, 0.0])
    return mesh.structured(
        [5, 5, 5],
        spacing=(0.25, 0.25, 0.25),
        scalars={"s": scalars},
        vectors={"velocity": vectors},
    )


_NATIVE_MISSING = mesh.problem(_rotation_dataset()).startswith(
    "gpui_toolkit native extension"
)


@unittest.skipIf(_NATIVE_MISSING, "native extension not built")
class MeshTests(unittest.TestCase):
    def test_structured_summary(self):
        summary = mesh.run(_rotation_dataset(), [])
        self.assertEqual(summary["points"], 125)
        self.assertEqual(summary["cells"], 64)
        self.assertEqual(mesh.problem(_rotation_dataset()), "")

    def test_vorticity_of_solid_rotation_is_two(self):
        summary = mesh.run(
            _rotation_dataset(),
            [{"op": "vorticity_magnitude", "field": "velocity"}],
        )
        field = next(
            f for f in summary["fields"] if f["name"] == "vorticity_magnitude"
        )
        self.assertAlmostEqual(field["min"], 2.0, places=9)
        self.assertAlmostEqual(field["max"], 2.0, places=9)

    def test_slice_and_streamlines(self):
        summary = mesh.run(
            _rotation_dataset(),
            [mesh.slice_(normal=(0, 0, 1), point=(0, 0, 0.5))],
        )
        self.assertEqual(summary["cells"], 32)
        lines = mesh.streamlines(
            _rotation_dataset(), [(0.5, 0.1, 0.5)], step=0.05, max_steps=40
        )
        self.assertEqual(len(lines), 2)  # both directions per seed
        self.assertGreater(len(lines[0]), 10)

    def test_geometry_ops(self):
        arrows = mesh.geometry(
            _rotation_dataset(),
            [mesh.slice_(normal=(0, 0, 1), point=(0, 0, 0.5)),
             mesh.arrows(scale=0.4, stride=4)],
        )
        self.assertGreater(len(arrows["triangles"]), 0)
        # Closed arrows emit 34 triangles = 102 indices each
        # (gpui-dataset-transformation glyph::solid_arrows_are_closed_triangle_meshes).
        self.assertEqual(len(arrows["triangles"]) % 102, 0)
        self.assertIn("magnitude", arrows["fields"])

        tubes = mesh.geometry(
            _rotation_dataset(),
            [mesh.streamline_op([(0.5, 0.1, 0.5)], step=0.05, max_steps=30),
             mesh.tubes(radius=0.02, sides=4)],
        )
        self.assertGreater(len(tubes["triangles"]), 0)

        frames = mesh.particles(
            [_rotation_dataset(), _rotation_dataset()],
            [0.0, 10.0],
            [(0.5, 0.1, 0.5)],
            dt=0.1,
            frames=3,
            trail=5,
        )
        self.assertEqual(len(frames), 3)
        self.assertIn("age", frames[2]["fields"])
        self.assertGreater(len(frames[2]["lines"]), 0)

    def test_mesh_file_round_trip_and_pvd(self):
        import os
        import tempfile

        dataset = _rotation_dataset()
        with tempfile.TemporaryDirectory() as tmp:
            obj = os.path.join(tmp, "grid.obj")
            mesh.write_mesh(dataset, obj)
            back = mesh.read_mesh(obj)
            self.assertEqual(len(back["points"]), 125)

            vtu = os.path.join(tmp, "grid.vtu")
            mesh.write_mesh(dataset, vtu, compress=True)
            back = mesh.read_mesh(vtu)
            self.assertEqual(len(back["points"]), 125)
            self.assertIn("s", back["fields"])

            pvd = os.path.join(tmp, "series.pvd")
            with open(os.path.join(tmp, "step.vtu"), "wb") as handle:
                with open(vtu, "rb") as source:
                    handle.write(source.read())
            with open(pvd, "w", encoding="utf-8") as handle:
                handle.write(
                    '<VTKFile type="Collection"><Collection>'
                    '<DataSet timestep="1.25" part="0" file="step.vtu"/>'
                    "</Collection></VTKFile>"
                )
            series = mesh.read_pvd(pvd)
            self.assertEqual(series["times"], [1.25])
            self.assertEqual(len(series["steps"]), 1)
            self.assertEqual(len(series["steps"][0]["points"]), 125)

            with self.assertRaises(ValueError):
                mesh.write_mesh(dataset, os.path.join(tmp, "grid.bogus"))

    def test_errors_are_reported(self):
        with self.assertRaises(ValueError):
            mesh.run(_rotation_dataset(), [{"op": "explode"}])
        bad = _rotation_dataset()
        bad["structured"]["dims"] = [0, 2, 2]
        self.assertNotEqual(mesh.problem(bad), "")


class MeshSpecTests(unittest.TestCase):
    def test_spec_shape_without_native(self):
        spec = _rotation_dataset()
        self.assertEqual(spec["structured"]["dims"], [5, 5, 5])
        self.assertEqual(len(spec["fields"]), 2)
        op = mesh.clip(normal=(1, 0, 0), keep="negative")
        self.assertEqual(op["op"], "clip")
        self.assertEqual(mesh.tubes(radius=0.1)["op"], "streamtubes")
        tensor_spec = mesh.structured(
            [2, 2, 2], tensors={"s": [[1, 0, 0, 0, 0, 0]] * 8}
        )
        self.assertEqual(tensor_spec["fields"][0]["kind"], "tensor")


if __name__ == "__main__":
    unittest.main()
