(() => {
  const query = new URLSearchParams(window.location.search);
  if (!query.has("scheduling-baseline")) return;

  const requestedSamples = Number(query.get("samples") ?? 240);
  const samples = Number.isFinite(requestedSamples)
    ? Math.min(2_000, Math.max(30, Math.trunc(requestedSamples)))
    : 240;
  const warmupSamples = Math.min(30, Math.max(5, Math.trunc(samples / 10)));

  const output = document.createElement("pre");
  output.id = "gpui-wasm-scheduling-baseline";
  output.setAttribute("aria-live", "polite");
  Object.assign(output.style, {
    position: "fixed",
    inset: "12px",
    zIndex: "2147483647",
    overflow: "auto",
    margin: "0",
    padding: "12px",
    color: "#cdd6f4",
    background: "rgba(17, 17, 27, 0.96)",
    font: "12px/1.45 ui-monospace, SFMono-Regular, Menlo, monospace",
    whiteSpace: "pre-wrap",
  });
  output.textContent = "Collecting wasm scheduling baseline…";
  document.body.append(output);
  document.documentElement.dataset.gpuiSchedulingBaseline = "running";

  function percentile(sorted, fraction) {
    const index = Math.min(
      sorted.length - 1,
      Math.max(0, Math.ceil(sorted.length * fraction) - 1),
    );
    return sorted[index];
  }

  function summarize(values) {
    const sorted = values.toSorted((left, right) => left - right);
    const sum = sorted.reduce((total, value) => total + value, 0);
    return {
      samples: sorted.length,
      min: sorted[0],
      median: percentile(sorted, 0.5),
      mean: sum / sorted.length,
      p95: percentile(sorted, 0.95),
      p99: percentile(sorted, 0.99),
      max: sorted.at(-1),
    };
  }

  function messageChannelDelay() {
    return new Promise((resolve, reject) => {
      const started = performance.now();
      let channel;
      try {
        channel = new MessageChannel();
      } catch (error) {
        reject(error);
        return;
      }
      channel.port1.onmessage = () => {
        const elapsed = performance.now() - started;
        channel.port1.close();
        channel.port2.close();
        resolve(elapsed);
      };
      channel.port2.postMessage(null);
    });
  }

  function timeoutDelay() {
    return new Promise((resolve) => {
      const started = performance.now();
      setTimeout(() => resolve(performance.now() - started), 0);
    });
  }

  function animationFrameTimestamp() {
    return new Promise((resolve) => requestAnimationFrame(resolve));
  }

  async function collectDelays(schedule) {
    for (let index = 0; index < warmupSamples; index += 1) await schedule();
    const values = [];
    for (let index = 0; index < samples; index += 1) values.push(await schedule());
    return summarize(values);
  }

  async function collectFrameIntervals() {
    let previous = await animationFrameTimestamp();
    const values = [];
    for (let index = 0; index < samples; index += 1) {
      const current = await animationFrameTimestamp();
      values.push(current - previous);
      previous = current;
    }
    return summarize(values);
  }

  async function collectDispatchToFrame() {
    const values = [];
    for (let index = 0; index < samples; index += 1) {
      const started = performance.now();
      await messageChannelDelay();
      const frameTimestamp = await animationFrameTimestamp();
      values.push(frameTimestamp - started);
    }
    return summarize(values);
  }

  async function collect() {
    // Wait for the wasm module and first GPUI paint before sampling.
    if (document.readyState !== "complete") {
      await new Promise((resolve) => window.addEventListener("load", resolve, { once: true }));
    }
    await animationFrameTimestamp();
    await animationFrameTimestamp();

    return {
      schema: "gpui-wasm-scheduling-baseline/v1",
      captured_at: new Date().toISOString(),
      page: window.location.pathname,
      environment: {
        user_agent: navigator.userAgent,
        platform: navigator.userAgentData?.platform ?? navigator.platform,
        hardware_concurrency: navigator.hardwareConcurrency,
        cross_origin_isolated: window.crossOriginIsolated,
        device_pixel_ratio: window.devicePixelRatio,
        viewport_css_px: [window.innerWidth, window.innerHeight],
      },
      units: "milliseconds",
      warmup_samples: warmupSamples,
      message_channel_dispatch: await collectDelays(messageChannelDelay),
      set_timeout_0_reference: await collectDelays(timeoutDelay),
      animation_frame_interval: await collectFrameIntervals(),
      message_channel_to_animation_frame: await collectDispatchToFrame(),
    };
  }

  const baselinePromise = collect();
  window.__gpuiSchedulingBaseline = baselinePromise;
  baselinePromise.then(
    (baseline) => {
      window.__gpuiSchedulingBaseline = baseline;
      output.textContent = JSON.stringify(baseline, null, 2);
      document.documentElement.dataset.gpuiSchedulingBaseline = "complete";
      console.info("[gpui-wasm-scheduling-baseline]", baseline);
    },
    (error) => {
      output.textContent = `Scheduling baseline failed: ${error?.stack ?? error}`;
      document.documentElement.dataset.gpuiSchedulingBaseline = "failed";
      console.error("[gpui-wasm-scheduling-baseline]", error);
    },
  );
})();
