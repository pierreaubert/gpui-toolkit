import unittest
from gpui_toolkit.audio import AudioAutomationPattern, AudioAutomationPatternReport, MeterSample, MeterStream, ScaleType, SpectrumSample, SpectrumStream, TickConfig
class AudioDeclarationsTests(unittest.TestCase):
 def test_scale_math_matches_native_tick_algorithm(self):
  self.assertEqual(ScaleType.LINEAR.value_to_position(50, 0, 100), .5)
  midpoint = ScaleType.QUADRATIC.value_to_position(50, 0, 100)
  self.assertLess(midpoint, .5)
  self.assertAlmostEqual(ScaleType.QUADRATIC.position_to_value(midpoint, 0, 100), 50)
  self.assertTrue(any(item.is_major for item in TickConfig().generate_ticks()))
 def test_pattern_reports_validate_the_stable_contract(self):
  pattern = AudioAutomationPattern("gain", "gain", "Potentiometer", "linear", (), (), "summary", "tests")
  self.assertEqual(AudioAutomationPatternReport(1, "report", (pattern,)).pattern("gain"), pattern)
 def test_real_time_streams_coalesce_latest_finite_sample(self):
  meter=MeterStream(); meter.push(MeterSample((-.1,))); meter.push(MeterSample((-.2,)))
  self.assertEqual(meter.take_latest().levels,(-.2,)); self.assertEqual(meter.dropped_samples,1)
  spectrum=SpectrumStream(); spectrum.push(SpectrumSample((20.,),(-30.,),48000.)); self.assertIsNotNone(spectrum.take_latest())
  with self.assertRaises(ValueError): MeterSample((float("nan"),))
if __name__ == "__main__": unittest.main()
