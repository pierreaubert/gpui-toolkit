# Decision: CUDA (Phase D) and MLX (Phase E) backend experiments

Date: 2026-11-13
Step: audit-20261113 §9 steps 16-17

## Phase D — CUDA: NO-GO (deferred, entry criteria unmet)

Evidence available at decision time:

- No NVIDIA hardware is available in the development or CI
  environment used for this audit (`nvidia-smi`/`nvcc` absent; the
  only GPU is Apple M4 Pro / Metal via wgpu). A CUDA experiment that
  cannot execute its differential or size-sweep benchmarks would be
  speculation, which the step's exit evidence explicitly forbids.
- The portable baseline it must beat now exists and is measured:
  flow diagnostics (`PreparedFlow`, resident outputs, M4 Pro) and GPU
  threshold (classify/scan/compact; one-shot 4.4 ms vs CPU 1.7 ms at
  48^3, upload-bound). No measured wgpu bottleneck attributable to the
  backend (rather than transfers) has been recorded yet.
- Cost side is real: `cudarc` + a second kernel set, driver/toolchain
  packaging per platform, and a second numerics surface to keep
  differential-tested.

Revisit when all of these hold: (1) NVIDIA hardware is available to
run the experiment branch's benches; (2) a profiled wgpu bottleneck
on that hardware is recorded against the Phase C baseline; (3) a
named product target ships to NVIDIA-only hosts. Until then the
portable wgpu path is the only GPU backend, and comments/docs say so
(no "CUDA-class" labels remain in the tree).

## Phase E — MLX: DEFERRED (experiment specified, not run)

MLX could in principle share unified-memory buffers with Metal on
Apple Silicon and skip wgpu's readback/copy path. What is known here:

- This machine is Apple Silicon, but no MLX runtime is installed
  (`python3 -c "import mlx"` fails) and the Rust MLX bindings would
  be a new native dependency with its own packaging story.
- The workload that would benefit (field -> diagnostics -> render
  without CPU readback) now exists in wgpu form: `PreparedFlow`
  resident outputs consumed GPU-to-GPU on a shared device, with
  readback counters proving zero CPU copies. The remaining wgpu cost
  on that path is dispatch/upload, not readback — exactly the cost
  MLX would also pay in kernel launches, minus buffer copies that
  the shared-device path already avoids.
- The measured one-shot GPU threshold shows the current bottleneck
  is host->device transfer, which unified memory could remove. That
  is a hypothesis, not a measurement.

Experiment to run when prioritized (one week-box): implement the
threshold classify kernel in MLX behind the same
`threshold_cells` contract, differential-test against CPU/wgpu on
the existing corpus, and size-sweep 24^3/48^3/96^3 including a
resident variant that consumes the output without CPU readback.
Adopt only if it beats wgpu by >= 2x on the resident path at 48^3+
without numerics regressions; otherwise record the negative result
here and close Phase E. The misleading "MLX-class" labels for
wgpu-Metal were removed independent of this decision.
