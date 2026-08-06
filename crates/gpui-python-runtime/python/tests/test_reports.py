import unittest
from gpui_toolkit.reports import PlatformCapability, PlatformCapabilityMatrix, PlatformCapabilityStatus, PlatformEvidence
class ReportTests(unittest.TestCase):
 def test_platform_evidence_never_implies_release_readiness(self):
  item = PlatformCapability("ios", "iOS", "2", PlatformCapabilityStatus.SUPPORTED, PlatformCapabilityStatus.SUPPORTED, PlatformCapabilityStatus.PARTIAL, PlatformCapabilityStatus.PARTIAL, PlatformEvidence(True, False, False, False, False), "device QA")
  self.assertFalse(PlatformCapabilityMatrix(1, "report", "today", (item,)).all_release_ready)
if __name__ == "__main__": unittest.main()
