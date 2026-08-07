import unittest

from gpui_toolkit.commands import CommandResult
from gpui_toolkit.d3 import ArrayOperation, ArrayRequest, ZoomOperation, ZoomRequest, ZoomResult


class D3ZoomTests(unittest.TestCase):
    def test_zoom_request_is_typed_and_serializable(self):
        request = ZoomRequest(
            (0, 100), (-20, 20),
            [ZoomOperation.zoom_to((20, 80), (-10, 10)), ZoomOperation.back()],
        )
        self.assertEqual(request.to_spec()["operations"][0]["kind"], "zoom_to")
        self.assertEqual(request.to_spec()["operations"][1], {"kind": "back"})

    def test_zoom_result_requires_a_successful_host_response(self):
        result = CommandResult.from_wire(
            "zoom", {"ok": True, "x": [20, 80], "y": [-10, 10], "zoomed": True, "level": 1, "back_results": []},
        )
        self.assertEqual(ZoomResult.from_command(result).x, (20.0, 80.0))
        with self.assertRaises(ValueError):
            ZoomRequest((1, 1), (0, 1))

    def test_array_requests_validate_native_operation_arguments(self):
        request = ArrayRequest(ArrayOperation.BISECT_RIGHT, [1, 2, 2, 4], value=2)
        self.assertEqual(request.to_spec()["operation"], "bisect_right")
        result = CommandResult.from_wire("bisect", {"ok": True, "value": 3})
        self.assertEqual(ArrayRequest.value_from_command(result), 3)
        with self.assertRaises(ValueError):
            ArrayRequest(ArrayOperation.QUANTILE, [1, 2]).to_spec()
