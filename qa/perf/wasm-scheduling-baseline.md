# Wasm scheduling baseline

The opt-in hello-web harness measures the browser primitives used by the GPUI
wasm dispatcher and frame pump. It reports sequential `MessageChannel`
dispatch latency, a `setTimeout(0)` reference, animation-frame intervals, and
`MessageChannel`-to-frame latency.

Run:

```bash
just wasm-serve-hello
```

Then open:

```text
http://127.0.0.1:8080/?scheduling-baseline=1&samples=240
```

The page displays machine-readable JSON and stores the same value in
`window.__gpuiSchedulingBaseline`. Keep the tab visible and avoid interacting
with the machine while sampling. Record the complete JSON below together with
the source revision; browser scheduling and refresh rate are environment
dependent.

## Current baseline

Captured from the current dirty `main` worktree at `5860d89` on 2026-08-23.
Firefox 154 ran on macOS with a 14-thread host, device pixel ratio 2, and a
cross-origin-isolated page. The visible tab used a nominal 60 Hz display.

The `MessageChannel` path had a 0 ms median and 0.02 ms p95 dispatch delay,
compared with 4.58 ms median and 5.04 ms p95 for `setTimeout(0)`. Animation
frames had a 16.66 ms median interval; dispatch-to-frame latency had a 16.46 ms
median and 17.20 ms p95.

```json
{
  "schema": "gpui-wasm-scheduling-baseline/v1",
  "captured_at": "2026-08-23T12:34:12.762Z",
  "page": "/",
  "environment": {
    "user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:154.0) Gecko/20100101 Firefox/154.0",
    "platform": "MacIntel",
    "hardware_concurrency": 14,
    "cross_origin_isolated": true,
    "device_pixel_ratio": 2,
    "viewport_css_px": [
      851,
      1486
    ]
  },
  "units": "milliseconds",
  "warmup_samples": 24,
  "message_channel_dispatch": {
    "samples": 240,
    "min": 0,
    "median": 0,
    "mean": 0.0065833333333333265,
    "p95": 0.020000000000003126,
    "p99": 0.0799999999999983,
    "max": 0.27999999999999403
  },
  "set_timeout_0_reference": {
    "samples": 240,
    "min": 4.019999999999982,
    "median": 4.579999999999927,
    "mean": 4.609416666666663,
    "p95": 5.039999999999964,
    "p99": 5.439999999999998,
    "max": 6.1400000000001
  },
  "animation_frame_interval": {
    "samples": 240,
    "min": 15.680000000000064,
    "median": 16.660000000000082,
    "mean": 16.666750000000008,
    "p95": 17.460000000000036,
    "p99": 17.659999999999854,
    "max": 17.66000000000031
  },
  "message_channel_to_animation_frame": {
    "samples": 240,
    "min": 9.38000000000011,
    "median": 16.459999999999127,
    "mean": 16.368666666666623,
    "p95": 17.199999999999818,
    "p99": 17.419999999999163,
    "max": 17.5
  }
}
```
