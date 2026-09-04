//! D3.js reference values for the many-body (charge) force golden test.
//!
//! Generates `force/many_body.json` from the real d3-force implementation
//! (d3 dependency, pinned in package.json). The Rust side
//! (`tests/force_many_body_golden.rs`) replays the same configuration and
//! tick counts through `d3rs::force` and compares numerically.
//!
//! Usage: `cd golden && node force_manybody.mjs`
//!
//! Configuration is identical on both sides on purpose:
//!   strength -30, theta 0.9, distanceMin 0, distanceMax Infinity,
//!   alpha 1 -> decayed per tick, alphaTarget 0, velocity decay x0.6.
//! distanceMin is 0 (not the d3 default 1) so the close-range softening rule
//! — which differs between implementations — is never exercised; node
//! spacings below are all >> 1 unit apart. Node coordinates use the same
//! golden-ratio lattice as the Rust benches and unit tests.

import * as d3 from "d3";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const d3Version = JSON.parse(
  fs.readFileSync(path.join(__dirname, "node_modules", "d3", "package.json"), "utf8"),
).version;

const PHI1 = 0.618033988749895;
const PHI2 = 0.381966011250105;

function lattice(n) {
  const nodes = [];
  for (let i = 0; i < n; i++) {
    nodes.push({
      index: i,
      x: ((i * PHI1) % 1) * 100.0,
      y: ((i * PHI2) % 1) * 100.0,
      vx: 0,
      vy: 0,
    });
  }
  return nodes;
}

function run_case(name, n, ticks) {
  const nodes = lattice(n);
  const sim = d3
    .forceSimulation(nodes)
    .alpha(1)
    .alphaMin(0.001)
    .alphaTarget(0)
    .velocityDecay(0.4) // internal multiplier 0.6, same as d3rs
    .force(
      "charge",
      d3.forceManyBody().strength(-30).theta(0.9).distanceMin(0).distanceMax(Infinity),
    )
    .stop(); // no async timer ticks; we tick manually below
  sim.tick(ticks);
  return {
    name,
    config: {
      strength: -30,
      theta: 0.9,
      distance_min: 0,
      distance_max: null, // Infinity; JSON has no Infinity
      alpha: 1,
      alpha_min: 0.001,
      alpha_target: 0,
      velocity_decay_multiplier: 0.6,
    },
    ticks,
    inputs: nodes.map((nd, i) => ({
      index: i,
      x: ((i * PHI1) % 1) * 100.0,
      y: ((i * PHI2) % 1) * 100.0,
    })),
    outputs: nodes.map((nd) => ({ x: nd.x, y: nd.y, vx: nd.vx, vy: nd.vy })),
  };
}

const golden = {
  module: "d3-force",
  function: "forceManyBody",
  source: "force_manybody.mjs (d3.forceSimulation + d3.forceManyBody)",
  d3_version: d3Version,
  // Acceptance tolerances asserted by the Rust test. BH matches d3's own
  // Barnes-Hut grouping to ~2e-13 here, so 1e-9 is strict but robust;
  // brute force differs by pure approximation error (~1.32 here), so its
  // tolerance only pins the fallback to the same basin, not parity.
  tolerance_suggested_bh: 1e-9,
  tolerance_suggested_brute: 5.0,
  test_cases: [run_case("single_tick_8", 8, 1), run_case("five_ticks_24", 24, 5)],
};

fs.mkdirSync(path.join(__dirname, "force"), { recursive: true });
fs.writeFileSync(
  path.join(__dirname, "force", "many_body.json"),
  JSON.stringify(golden, null, 2) + "\n",
);
console.log(`wrote force/many_body.json (d3 ${d3Version})`);
