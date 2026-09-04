//! Frame sequences for the buffer-diff golden equivalence test.
//!
//! Generates `gpu/buffer_diff.json`: per-frame 12-float composite-uniform
//! payloads (`[dst_origin(2), dst_size(2), src_origin(2), src_size(2),
//! tex_size(2), target_size(2)]`, matching `vello2d::wgpu_draw`) across three
//! scenarios: a static rect, a continuous pan, and a pan that settles.
//! The Rust side (`tests/buffer_diff_golden.rs`) replays each sequence
//! through the full-upload path and through `mesh::BufferUploadCache` and
//! asserts byte-identical delivery with the frozen `expected_writes` count.
//!
//! Usage: `cd golden && node gpu_buffer_diff.mjs`

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function uniforms(dstOrigin, dstSize, srcOrigin, srcSize, texSize, targetSize) {
  return [...dstOrigin, ...dstSize, ...srcOrigin, ...srcSize, ...texSize, ...targetSize];
}

const DST = [800, 600];
const SRC = [800, 600];
const TEX = [800, 600];
const TARGET = [1600, 1200];

const STATIC_FRAME = uniforms([0, 0], DST, [0, 0], SRC, TEX, TARGET);

function panFrame(i) {
  // Horizontal scroll: the visible sub-rectangle slides each frame.
  const o = i * 16.0;
  return uniforms([o, 0], DST, [o, 0], SRC, TEX, TARGET);
}

const staticCase = {
  name: "static_30",
  description: "Camera at rest: 30 identical frames, exactly 1 queue write.",
  frames: Array.from({ length: 30 }, () => STATIC_FRAME),
  expected_writes: 1,
};

const panCase = {
  name: "pan_20",
  description: "Continuous pan: every frame differs, 20 queue writes.",
  frames: Array.from({ length: 20 }, (_, i) => panFrame(i)),
  expected_writes: 20,
};

const settleFrames = Array.from({ length: 10 }, (_, i) => panFrame(i));
const settled = panFrame(9);
for (let i = 0; i < 15; i++) settleFrames.push(settled);
const settleCase = {
  name: "settle_25",
  description: "Pan for 10 frames then hold: 10 writes, 15 skipped.",
  frames: settleFrames,
  expected_writes: 10,
};

const golden = {
  module: "d3rs-buffer-diff",
  function: "BufferUploadCache",
  source: "gpu_buffer_diff.mjs (synthetic composite-uniform frame sequences)",
  test_cases: [staticCase, panCase, settleCase],
};

fs.mkdirSync(path.join(__dirname, "gpu"), { recursive: true });
fs.writeFileSync(
  path.join(__dirname, "gpu", "buffer_diff.json"),
  JSON.stringify(golden, null, 2) + "\n",
);
console.log("wrote gpu/buffer_diff.json");
