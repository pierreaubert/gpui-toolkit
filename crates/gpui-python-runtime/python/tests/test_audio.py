import unittest
from threading import Event
from gpui_toolkit.audio import AudioAutomationPattern, AudioAutomationPatternReport, AudioControlScale, AudioControlSize, MeterSample, MeterStream, ScaleType, SpectrumSample, SpectrumStream, TickConfig, accessibility_from_command, horizontal_meter, level_meter, potentiometer, reports_from_command, spectrum, vertical_slider, volume_knob
from gpui_toolkit.commands import CommandResult
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
 def test_native_audio_controls_and_visualizers_are_declarative(self):
  nodes=(
   potentiometer(id="gain", value=.5, minimum=0, maximum=1, label="Gain", unit="dB", size=AudioControlSize.SM, action="preview", commit_action="commit"),
   vertical_slider(id="fader", value=1000, minimum=20, maximum=20000, scale=AudioControlScale.LOGARITHMIC, with_ticks=True, peak=1200),
   volume_knob(id="volume", value=.7, muted=True, action="preview", commit_action="commit", mute_action="mute"),
   horizontal_meter(id="horizontal", levels=(-12,-6), peaks=(-3,-1), channel_names=("L","R")),
   level_meter(id="level", levels=(-12,-6), peaks=(-3,-1), channel_names=("L","R")),
   spectrum(id="spectrum", magnitudes=(-80,-40,-20), previous=(-90,-50,-30), smoothing=.6),
  )
  self.assertEqual([node.to_spec()["kind"] for node in nodes], ["audio_potentiometer","audio_vertical_slider","audio_volume_knob","audio_horizontal_meter","audio_level_meter","audio_spectrum"])
  self.assertEqual(nodes[1].to_spec()["scale"], "logarithmic")
  self.assertEqual(nodes[-1].to_spec()["magnitudes"], [-80.0,-40.0,-20.0])
 def test_native_audio_reports_decode_typed_evidence(self):
  payload={
   "ok":True,
   "automation":{"schema_version":1,"report_type":"gpui-audio-kit-automation-patterns","unique_ids":True,"patterns":[{"id":"gain","parameter_family":"gain","recommended_control":"Potentiometer","scale":"linear","automation_sources":["host"],"expected_interactions":["drag"],"accessibility_summary_contract":"slider summary","release_evidence":"native tests","status":"implemented"}],"markdown":"# Automation"},
   "visual":{"schema_version":1,"report_type":"visual","crate_name":"gpui-audio-kit","crate_version":"1.0.0","capture_count":2,"expected_capture_count":2,"unique_capture_ids":True,"components":["potentiometer","meter"],"markdown":"| capture |"},
   "design_tokens":{"knob_arc_start_deg":135,"knob_arc_sweep_deg":270,"knob_arc_widths":[2,3],"knob_arc_track_widths":[1,2],"knob_arc_glow":0,"knob_arc_segments":32,"knob_border_width":1,"knob_label_style":0,"knob_indicator_style":0,"slider_track_widths":[2,3],"meter_label_style":0,"meter_use_gradient":True,"meter_corner_radius":2,"meter_glow":0,"toggle_variant":0,"corner_radius":4,"min_touch_target":44,"control_padding_x":8,"control_padding_y":6,"animation_duration_ms":150,"prefer_spring":False,"spring_stiffness":200,"spring_damping":20},
  }
  reports=reports_from_command(CommandResult.from_wire("audio",payload))
  self.assertEqual(reports.automation.pattern("gain").status,"implemented")
  self.assertEqual(reports.visual.capture_count,reports.visual.expected_capture_count)
  self.assertEqual(reports.design_tokens.min_touch_target,44)
 def test_native_accessibility_summary_decodes(self):
  result=CommandResult.from_wire("a11y",{"ok":True,"summaries":[{"control_type":"potentiometer","label":"Gain","role":"slider","value_now":.5,"value_min":0,"value_max":1,"value_text":"0.5 dB","unit":"dB","normalized":.5,"scale":"linear","selected":True,"disabled":False,"muted":False,"peak_value":None,"description":"Gain control"}]})
  summary=accessibility_from_command(result)[0]
  self.assertEqual((summary.role,summary.scale,summary.normalized),("slider","linear",.5))
 def test_native_stream_sends_bounded_binary_frame_and_releases(self):
  class Context:
   def __init__(self): self.frame=None; self.released=None; self.ready=Event()
   def resource_frame(self,header,payload): self.frame=(header,payload); self.ready.set()
   def drop_resource(self,id,generation): self.released=(id,generation)
  context=Context(); stream=MeterStream(context,"meter",channel_count=2,sample_rate=48000)
  stream.push(MeterSample((-12.,-6.),(-3.,-1.)))
  self.assertTrue(context.ready.wait(.5)); stream.close()
  header,payload=context.frame
  self.assertEqual((header["frame_kind"],header["shape"],len(payload)),("meter",[2,2],16))
  self.assertEqual(context.released,("meter",1))
if __name__ == "__main__": unittest.main()
