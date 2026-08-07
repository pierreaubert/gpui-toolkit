import unittest
from gpui_toolkit.reports import CrateStability, DependencyHygieneCheck, DependencyHygieneReport, DependencyHygieneStatus, PlatformCapability, PlatformCapabilityMatrix, PlatformCapabilityStatus, PlatformEvidence, PublishPlan, PublishPlanEntry, PublishPlanStatus, ReleaseNotesArtifact, ReleaseNotesArtifactReport, ReleaseNotesArtifactStatus, ReleaseNotesEntry, ReleaseNotesReport, ReleaseNotesStatus, ReleasePackagingEntry, ReleasePackagingReport, ReleasePackagingStatus, StabilityLevel, StabilityReport, VendoredPatch, VendoredPatchMaintenance, VendoredPatchManifest, VendoredPatchStatus, PublishDecision, AggregateFeature
class ReportTests(unittest.TestCase):
 def test_platform_evidence_never_implies_release_readiness(self):
  item = PlatformCapability("ios", "iOS", "2", PlatformCapabilityStatus.SUPPORTED, PlatformCapabilityStatus.SUPPORTED, PlatformCapabilityStatus.PARTIAL, PlatformCapabilityStatus.PARTIAL, PlatformEvidence(True, False, False, False, False), "device QA")
  self.assertFalse(PlatformCapabilityMatrix(1, "report", "today", (item,)).all_release_ready)
 def test_release_reports_keep_pending_work_blocking(self):
  check = DependencyHygieneCheck("audit", "cargo deny check", DependencyHygieneStatus.RELEASE_RUN_PENDING, "audit", "none", "run")
  hygiene = DependencyHygieneReport(1, "hygiene", "today", "deny.toml", (check,), ())
  self.assertFalse(hygiene.all_release_ready); self.assertEqual(hygiene.blocking_checks, (check,))
  pending = PublishPlanEntry("gpui-ui-kit", 1, "core", "cargo publish", PublishPlanStatus.PENDING_DRY_RUN, "release", "none", "run")
  self.assertFalse(PublishPlan(1, "publish", "today", (pending,)).all_release_ready)
  packaging = ReleasePackagingEntry("wheel", "python", "tooling", "build", ReleasePackagingStatus.EXTERNAL_GATE, "none", "sign")
  self.assertFalse(ReleasePackagingReport(1, "packaging", "today", (packaging,)).all_release_ready)
 def test_complete_aggregate_report_family_is_typed_and_read_only(self):
  stability = CrateStability("gpui-ui-kit", AggregateFeature.UI, StabilityLevel.BETA, PublishDecision.BETA_AFTER_GATES, "qa", "note")
  self.assertTrue(StabilityReport(1, "stability", "today", (stability,)).all_release_ready)
  note = ReleaseNotesEntry("gpui-ui-kit", "beta", ReleaseNotesStatus.READY, "beta", "desktop", "notes", "present", "none")
  self.assertTrue(ReleaseNotesReport(1, "notes", "today", (note,)).all_release_ready)
  artifact = ReleaseNotesArtifact("visual", "gpui-ui-kit", "visual", "manifest", ReleaseNotesArtifactStatus.EXTERNAL_GATE, "none", "capture")
  self.assertEqual(ReleaseNotesArtifactReport(1, "artifacts", "today", (artifact,)).blocking_artifacts, (artifact,))
  patch = VendoredPatch("gpui", "vendor/gpui", "https://example.test", "rev", "ui", "today", 30, "upstream", "diff", "rev", VendoredPatchStatus.ACTIVE_PATCH, VendoredPatchMaintenance.SCRIPT_VENDORED, "patch", ("fix",), "test", "VENDORED.md")
  self.assertEqual(VendoredPatchManifest(1, "patches", (patch,)).active_patches, (patch,))
if __name__ == "__main__": unittest.main()
