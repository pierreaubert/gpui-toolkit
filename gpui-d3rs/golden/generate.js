/**
 * Golden file generator for d3rs compatibility testing
 *
 * This script generates JSON golden files containing D3.js outputs
 * that can be compared against Rust implementations.
 *
 * Usage:
 *   node generate.js           # Generate all golden files
 *   node generate.js scales    # Generate only scale tests
 *   node generate.js interpolate
 *   node generate.js array
 *   node generate.js color
 *   node generate.js shape
 */

const d3 = require('d3');
const fs = require('fs');
const path = require('path');

const TOLERANCE = 1e-6;

// Utility to create golden file structure
function createGoldenFile(module, func, testCases) {
  return {
    module,
    function: func,
    d3_version: d3.version,
    tolerance: TOLERANCE,
    generated_at: new Date().toISOString(),
    test_cases: testCases
  };
}

// ============================================================================
// SCALE GENERATORS
// ============================================================================

function generateLinearScaleTests() {
  const testCases = [];

  // Basic domain/range
  {
    const scale = d3.scaleLinear().domain([0, 100]).range([0, 500]);
    const inputs = [0, 25, 50, 75, 100];
    testCases.push({
      name: "basic_domain_range",
      config: { domain: [0, 100], range: [0, 500] },
      inputs,
      outputs: inputs.map(v => scale(v)),
      invert_inputs: [0, 125, 250, 375, 500],
      invert_outputs: [0, 125, 250, 375, 500].map(v => scale.invert(v))
    });
  }

  // Inverted range
  {
    const scale = d3.scaleLinear().domain([0, 100]).range([500, 0]);
    const inputs = [0, 50, 100];
    testCases.push({
      name: "inverted_range",
      config: { domain: [0, 100], range: [500, 0] },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Negative domain
  {
    const scale = d3.scaleLinear().domain([-100, 100]).range([0, 1]);
    const inputs = [-100, -50, 0, 50, 100];
    testCases.push({
      name: "negative_domain",
      config: { domain: [-100, 100], range: [0, 1] },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Extrapolation (values outside domain)
  {
    const scale = d3.scaleLinear().domain([0, 100]).range([0, 500]);
    const inputs = [-50, 150];
    testCases.push({
      name: "extrapolation",
      config: { domain: [0, 100], range: [0, 500], clamp: false },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Clamped
  {
    const scale = d3.scaleLinear().domain([0, 100]).range([0, 500]).clamp(true);
    const inputs = [-50, 0, 50, 100, 150];
    testCases.push({
      name: "clamped",
      config: { domain: [0, 100], range: [0, 500], clamp: true },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Nice domain
  {
    const scale = d3.scaleLinear().domain([0.123, 0.987]).nice();
    testCases.push({
      name: "nice_domain",
      config: { domain: [0.123, 0.987] },
      nice_domain: scale.domain()
    });
  }

  // Ticks
  {
    const scale = d3.scaleLinear().domain([0, 100]);
    testCases.push({
      name: "ticks_default",
      config: { domain: [0, 100] },
      ticks_count: 10,
      ticks: scale.ticks(10)
    });
  }

  {
    const scale = d3.scaleLinear().domain([0, 100]);
    testCases.push({
      name: "ticks_5",
      config: { domain: [0, 100] },
      ticks_count: 5,
      ticks: scale.ticks(5)
    });
  }

  // Float precision
  {
    const scale = d3.scaleLinear().domain([0.0, 1.0]).range([0.0, 100.0]);
    const inputs = [0.0, 0.1, 0.25, 0.333333, 0.5, 0.666666, 0.75, 0.9, 1.0];
    testCases.push({
      name: "float_precision",
      config: { domain: [0, 1], range: [0, 100] },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  const golden = createGoldenFile("d3-scale", "scaleLinear", testCases);
  fs.writeFileSync(path.join(__dirname, 'scales', 'linear.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: scales/linear.json');
}

function generateLogScaleTests() {
  const testCases = [];

  // Basic log scale
  {
    const scale = d3.scaleLog().domain([1, 1000]).range([0, 1]);
    const inputs = [1, 10, 100, 1000];
    testCases.push({
      name: "basic_log",
      config: { domain: [1, 1000], range: [0, 1], base: 10 },
      inputs,
      outputs: inputs.map(v => scale(v)),
      invert_inputs: [0, 0.25, 0.5, 0.75, 1],
      invert_outputs: [0, 0.25, 0.5, 0.75, 1].map(v => scale.invert(v))
    });
  }

  // Frequency range (audio)
  {
    const scale = d3.scaleLog().domain([20, 20000]).range([0, 1]);
    const inputs = [20, 100, 1000, 10000, 20000];
    testCases.push({
      name: "frequency_range",
      config: { domain: [20, 20000], range: [0, 1], base: 10 },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Base 2
  {
    const scale = d3.scaleLog().base(2).domain([1, 16]).range([0, 1]);
    const inputs = [1, 2, 4, 8, 16];
    testCases.push({
      name: "base_2",
      config: { domain: [1, 16], range: [0, 1], base: 2 },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Base e (natural log)
  {
    const scale = d3.scaleLog().base(Math.E).domain([1, Math.E * Math.E]).range([0, 1]);
    const inputs = [1, Math.E, Math.E * Math.E];
    testCases.push({
      name: "base_e",
      config: { domain: [1, Math.E * Math.E], range: [0, 1], base: Math.E },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Ticks
  {
    const scale = d3.scaleLog().domain([1, 1000]);
    testCases.push({
      name: "ticks",
      config: { domain: [1, 1000], base: 10 },
      ticks: scale.ticks()
    });
  }

  const golden = createGoldenFile("d3-scale", "scaleLog", testCases);
  fs.writeFileSync(path.join(__dirname, 'scales', 'log.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: scales/log.json');
}

function generatePowScaleTests() {
  const testCases = [];

  // Square (exponent 2)
  {
    const scale = d3.scalePow().exponent(2).domain([0, 10]).range([0, 100]);
    const inputs = [0, 1, 2, 3, 5, 10];
    testCases.push({
      name: "exponent_2",
      config: { domain: [0, 10], range: [0, 100], exponent: 2 },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Sqrt (exponent 0.5)
  {
    const scale = d3.scaleSqrt().domain([0, 100]).range([0, 10]);
    const inputs = [0, 1, 4, 9, 16, 25, 100];
    testCases.push({
      name: "sqrt",
      config: { domain: [0, 100], range: [0, 10], exponent: 0.5 },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Cubic (exponent 3)
  {
    const scale = d3.scalePow().exponent(3).domain([0, 10]).range([0, 1000]);
    const inputs = [0, 1, 2, 5, 10];
    testCases.push({
      name: "exponent_3",
      config: { domain: [0, 10], range: [0, 1000], exponent: 3 },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Invert
  {
    const scale = d3.scalePow().exponent(2).domain([0, 10]).range([0, 100]);
    const inputs = [0, 25, 100];
    testCases.push({
      name: "invert",
      config: { domain: [0, 10], range: [0, 100], exponent: 2 },
      invert_inputs: inputs,
      invert_outputs: inputs.map(v => scale.invert(v))
    });
  }

  const golden = createGoldenFile("d3-scale", "scalePow", testCases);
  fs.writeFileSync(path.join(__dirname, 'scales', 'pow.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: scales/pow.json');
}

function generateQuantizeScaleTests() {
  const testCases = [];

  // Basic quantize
  {
    const scale = d3.scaleQuantize().domain([0, 100]).range(['a', 'b', 'c', 'd']);
    const inputs = [0, 12, 25, 37, 50, 62, 75, 87, 100];
    testCases.push({
      name: "basic",
      config: { domain: [0, 100], range: ['a', 'b', 'c', 'd'] },
      inputs,
      outputs: inputs.map(v => scale(v)),
      thresholds: scale.thresholds()
    });
  }

  // Numeric range
  {
    const scale = d3.scaleQuantize().domain([0, 1]).range([0, 1, 2, 3, 4]);
    const inputs = [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
    testCases.push({
      name: "numeric_range",
      config: { domain: [0, 1], range: [0, 1, 2, 3, 4] },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Invert extent
  {
    const scale = d3.scaleQuantize().domain([0, 100]).range(['low', 'medium', 'high']);
    testCases.push({
      name: "invert_extent",
      config: { domain: [0, 100], range: ['low', 'medium', 'high'] },
      invert_extent: {
        low: scale.invertExtent('low'),
        medium: scale.invertExtent('medium'),
        high: scale.invertExtent('high')
      }
    });
  }

  const golden = createGoldenFile("d3-scale", "scaleQuantize", testCases);
  fs.writeFileSync(path.join(__dirname, 'scales', 'quantize.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: scales/quantize.json');
}

function generateQuantileScaleTests() {
  const testCases = [];

  // Basic quantile
  {
    const data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const scale = d3.scaleQuantile().domain(data).range(['q1', 'q2', 'q3', 'q4']);
    const inputs = [1, 3, 5, 7, 10];
    testCases.push({
      name: "basic",
      config: { domain: data, range: ['q1', 'q2', 'q3', 'q4'] },
      inputs,
      outputs: inputs.map(v => scale(v)),
      quantiles: scale.quantiles()
    });
  }

  // With outliers
  {
    const data = [1, 1, 1, 2, 2, 3, 3, 5, 10, 100];
    const scale = d3.scaleQuantile().domain(data).range(['low', 'medium', 'high']);
    testCases.push({
      name: "with_outliers",
      config: { domain: data, range: ['low', 'medium', 'high'] },
      quantiles: scale.quantiles()
    });
  }

  const golden = createGoldenFile("d3-scale", "scaleQuantile", testCases);
  fs.writeFileSync(path.join(__dirname, 'scales', 'quantile.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: scales/quantile.json');
}

function generateThresholdScaleTests() {
  const testCases = [];

  // Basic threshold
  {
    const scale = d3.scaleThreshold().domain([0, 1]).range(['negative', 'zero', 'positive']);
    const inputs = [-1, -0.5, 0, 0.5, 1, 2];
    testCases.push({
      name: "basic",
      config: { domain: [0, 1], range: ['negative', 'zero', 'positive'] },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  // Multiple thresholds
  {
    const scale = d3.scaleThreshold().domain([10, 20, 30, 40]).range(['F', 'D', 'C', 'B', 'A']);
    const inputs = [0, 10, 15, 20, 25, 30, 35, 40, 50];
    testCases.push({
      name: "grades",
      config: { domain: [10, 20, 30, 40], range: ['F', 'D', 'C', 'B', 'A'] },
      inputs,
      outputs: inputs.map(v => scale(v))
    });
  }

  const golden = createGoldenFile("d3-scale", "scaleThreshold", testCases);
  fs.writeFileSync(path.join(__dirname, 'scales', 'threshold.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: scales/threshold.json');
}

// ============================================================================
// INTERPOLATE GENERATORS
// ============================================================================

function generateInterpolateNumberTests() {
  const testCases = [];

  // Basic number interpolation
  {
    const interp = d3.interpolateNumber(0, 100);
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "basic",
      config: { a: 0, b: 100 },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Negative range
  {
    const interp = d3.interpolateNumber(-100, 100);
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "negative_range",
      config: { a: -100, b: 100 },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Extrapolation
  {
    const interp = d3.interpolateNumber(0, 100);
    const ts = [-0.5, 0, 0.5, 1, 1.5];
    testCases.push({
      name: "extrapolation",
      config: { a: 0, b: 100 },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Round
  {
    const interp = d3.interpolateRound(0, 100);
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "round",
      config: { a: 0, b: 100, round: true },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  const golden = createGoldenFile("d3-interpolate", "interpolateNumber", testCases);
  fs.writeFileSync(path.join(__dirname, 'interpolate', 'number.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: interpolate/number.json');
}

function generateInterpolateColorTests() {
  const testCases = [];

  // RGB interpolation
  {
    const interp = d3.interpolateRgb('red', 'blue');
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "rgb_red_blue",
      config: { a: 'red', b: 'blue', space: 'rgb' },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // HSL interpolation
  {
    const interp = d3.interpolateHsl('red', 'blue');
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "hsl_red_blue",
      config: { a: 'red', b: 'blue', space: 'hsl' },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // HSL long
  {
    const interp = d3.interpolateHslLong('red', 'blue');
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "hsl_long_red_blue",
      config: { a: 'red', b: 'blue', space: 'hsl-long' },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Lab interpolation
  {
    const interp = d3.interpolateLab('red', 'blue');
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "lab_red_blue",
      config: { a: 'red', b: 'blue', space: 'lab' },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // HCL interpolation
  {
    const interp = d3.interpolateHcl('red', 'blue');
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "hcl_red_blue",
      config: { a: 'red', b: 'blue', space: 'hcl' },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Cubehelix interpolation
  {
    const interp = d3.interpolateCubehelix('red', 'blue');
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "cubehelix_red_blue",
      config: { a: 'red', b: 'blue', space: 'cubehelix' },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Hex colors
  {
    const interp = d3.interpolateRgb('#ff0000', '#0000ff');
    const ts = [0, 0.5, 1];
    testCases.push({
      name: "rgb_hex",
      config: { a: '#ff0000', b: '#0000ff', space: 'rgb' },
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  const golden = createGoldenFile("d3-interpolate", "interpolateColor", testCases);
  fs.writeFileSync(path.join(__dirname, 'interpolate', 'color.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: interpolate/color.json');
}

// ============================================================================
// ARRAY GENERATORS
// ============================================================================

function generateArrayStatisticsTests() {
  const testCases = [];

  // min/max/extent
  {
    const data = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
    testCases.push({
      name: "min_max_extent",
      data,
      min: d3.min(data),
      max: d3.max(data),
      extent: d3.extent(data)
    });
  }

  // sum/mean/median
  {
    const data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    testCases.push({
      name: "sum_mean_median",
      data,
      sum: d3.sum(data),
      mean: d3.mean(data),
      median: d3.median(data)
    });
  }

  // variance/deviation
  {
    const data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    testCases.push({
      name: "variance_deviation",
      data,
      variance: d3.variance(data),
      deviation: d3.deviation(data)
    });
  }

  // quantile
  {
    const data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    testCases.push({
      name: "quantile",
      data,
      q0: d3.quantile(data, 0),
      q25: d3.quantile(data, 0.25),
      q50: d3.quantile(data, 0.5),
      q75: d3.quantile(data, 0.75),
      q100: d3.quantile(data, 1)
    });
  }

  // cumsum
  {
    const data = [1, 2, 3, 4, 5];
    testCases.push({
      name: "cumsum",
      data,
      cumsum: Array.from(d3.cumsum(data))
    });
  }

  // With accessor
  {
    const data = [{ value: 10 }, { value: 20 }, { value: 30 }];
    testCases.push({
      name: "with_accessor",
      data: data.map(d => d.value),
      min: d3.min(data, d => d.value),
      max: d3.max(data, d => d.value),
      mean: d3.mean(data, d => d.value)
    });
  }

  // Empty array
  {
    testCases.push({
      name: "empty_array",
      data: [],
      min: d3.min([]),
      max: d3.max([]),
      mean: d3.mean([])
    });
  }

  const golden = createGoldenFile("d3-array", "statistics", testCases);
  fs.writeFileSync(path.join(__dirname, 'array', 'statistics.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: array/statistics.json');
}

function generateArrayBisectTests() {
  const testCases = [];

  // Basic bisect
  {
    const arr = [1, 2, 3, 4, 5];
    testCases.push({
      name: "basic",
      array: arr,
      bisect_left: {
        0: d3.bisectLeft(arr, 0),
        1: d3.bisectLeft(arr, 1),
        2.5: d3.bisectLeft(arr, 2.5),
        5: d3.bisectLeft(arr, 5),
        6: d3.bisectLeft(arr, 6)
      },
      bisect_right: {
        0: d3.bisectRight(arr, 0),
        1: d3.bisectRight(arr, 1),
        2.5: d3.bisectRight(arr, 2.5),
        5: d3.bisectRight(arr, 5),
        6: d3.bisectRight(arr, 6)
      }
    });
  }

  // With duplicates
  {
    const arr = [1, 2, 2, 2, 3, 4, 5];
    testCases.push({
      name: "with_duplicates",
      array: arr,
      bisect_left_2: d3.bisectLeft(arr, 2),
      bisect_right_2: d3.bisectRight(arr, 2)
    });
  }

  // Float array
  {
    const arr = [0.1, 0.2, 0.3, 0.4, 0.5];
    testCases.push({
      name: "floats",
      array: arr,
      bisect_left_025: d3.bisectLeft(arr, 0.25),
      bisect_right_025: d3.bisectRight(arr, 0.25)
    });
  }

  const golden = createGoldenFile("d3-array", "bisect", testCases);
  fs.writeFileSync(path.join(__dirname, 'array', 'bisect.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: array/bisect.json');
}

function generateArrayBinTests() {
  const testCases = [];

  // Basic binning
  {
    const data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const bins = d3.bin().thresholds(5)(data);
    testCases.push({
      name: "basic",
      data,
      threshold_count: 5,
      bins: bins.map(b => ({
        x0: b.x0,
        x1: b.x1,
        length: b.length,
        values: Array.from(b)
      }))
    });
  }

  // Custom domain
  {
    const data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const bins = d3.bin().domain([0, 10]).thresholds(10)(data);
    testCases.push({
      name: "custom_domain",
      data,
      domain: [0, 10],
      threshold_count: 10,
      bins: bins.map(b => ({
        x0: b.x0,
        x1: b.x1,
        length: b.length
      }))
    });
  }

  // Sturges thresholds
  {
    const data = Array.from({ length: 100 }, (_, i) => i);
    const bins = d3.bin().thresholds(d3.thresholdSturges)(data);
    testCases.push({
      name: "sturges",
      data_length: 100,
      bin_count: bins.length
    });
  }

  const golden = createGoldenFile("d3-array", "bin", testCases);
  fs.writeFileSync(path.join(__dirname, 'array', 'bin.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: array/bin.json');
}

function generateArrayTicksTests() {
  const testCases = [];

  // Basic ticks
  {
    testCases.push({
      name: "basic_0_100",
      start: 0,
      stop: 100,
      count: 10,
      ticks: d3.ticks(0, 100, 10)
    });
  }

  // Negative range
  {
    testCases.push({
      name: "negative_range",
      start: -50,
      stop: 50,
      count: 10,
      ticks: d3.ticks(-50, 50, 10)
    });
  }

  // Small range
  {
    testCases.push({
      name: "small_range",
      start: 0,
      stop: 1,
      count: 5,
      ticks: d3.ticks(0, 1, 5)
    });
  }

  // Large range
  {
    testCases.push({
      name: "large_range",
      start: 0,
      stop: 1000000,
      count: 5,
      ticks: d3.ticks(0, 1000000, 5)
    });
  }

  // Nice
  {
    testCases.push({
      name: "nice",
      start: 0.123,
      stop: 0.987,
      count: 5,
      ticks: d3.ticks(0.123, 0.987, 5),
      nice: d3.nice(0.123, 0.987, 5)
    });
  }

  // tickStep
  {
    testCases.push({
      name: "tick_step",
      start: 0,
      stop: 100,
      count: 10,
      tick_step: d3.tickStep(0, 100, 10)
    });
  }

  const golden = createGoldenFile("d3-array", "ticks", testCases);
  fs.writeFileSync(path.join(__dirname, 'array', 'ticks.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: array/ticks.json');
}

// ============================================================================
// COLOR GENERATORS
// ============================================================================

function generateColorTests() {
  const testCases = [];

  // RGB parsing
  {
    const colors = ['red', 'green', 'blue', '#ff0000', '#00ff00', '#0000ff', 'rgb(255, 128, 0)', 'rgba(100, 150, 200, 0.5)'];
    testCases.push({
      name: "parsing",
      colors: colors.map(c => {
        const color = d3.color(c);
        return {
          input: c,
          r: color.r,
          g: color.g,
          b: color.b,
          opacity: color.opacity,
          hex: color.formatHex(),
          rgb: color.formatRgb()
        };
      })
    });
  }

  // HSL conversion
  {
    const colors = ['red', 'green', 'blue', 'yellow', 'cyan', 'magenta'];
    testCases.push({
      name: "hsl_conversion",
      colors: colors.map(c => {
        const rgb = d3.color(c);
        const hsl = d3.hsl(c);
        return {
          input: c,
          h: hsl.h,
          s: hsl.s,
          l: hsl.l
        };
      })
    });
  }

  // Brighter/darker
  {
    const base = d3.color('steelblue');
    testCases.push({
      name: "brighter_darker",
      base: base.formatHex(),
      brighter_1: base.brighter(1).formatHex(),
      brighter_2: base.brighter(2).formatHex(),
      darker_1: base.darker(1).formatHex(),
      darker_2: base.darker(2).formatHex()
    });
  }

  // Color schemes
  {
    testCases.push({
      name: "schemes",
      category10: d3.schemeCategory10,
      tableau10: d3.schemeTableau10,
      paired: d3.schemePaired,
      set1: d3.schemeSet1
    });
  }

  const golden = createGoldenFile("d3-color", "color", testCases);
  fs.writeFileSync(path.join(__dirname, 'color', 'color.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: color/color.json');
}

// ============================================================================
// SHAPE GENERATORS
// ============================================================================

function generateArcTests() {
  const testCases = [];

  // Basic arc
  {
    const arc = d3.arc()
      .innerRadius(0)
      .outerRadius(100)
      .startAngle(0)
      .endAngle(Math.PI / 2);

    testCases.push({
      name: "basic_quarter",
      config: {
        innerRadius: 0,
        outerRadius: 100,
        startAngle: 0,
        endAngle: Math.PI / 2
      },
      path: arc(),
      centroid: arc.centroid()
    });
  }

  // Donut arc
  {
    const arc = d3.arc()
      .innerRadius(50)
      .outerRadius(100)
      .startAngle(0)
      .endAngle(Math.PI);

    testCases.push({
      name: "donut_half",
      config: {
        innerRadius: 50,
        outerRadius: 100,
        startAngle: 0,
        endAngle: Math.PI
      },
      path: arc(),
      centroid: arc.centroid()
    });
  }

  // With corner radius
  {
    const arc = d3.arc()
      .innerRadius(50)
      .outerRadius(100)
      .cornerRadius(10)
      .startAngle(0)
      .endAngle(Math.PI / 2);

    testCases.push({
      name: "corner_radius",
      config: {
        innerRadius: 50,
        outerRadius: 100,
        cornerRadius: 10,
        startAngle: 0,
        endAngle: Math.PI / 2
      },
      path: arc()
    });
  }

  const golden = createGoldenFile("d3-shape", "arc", testCases);
  fs.writeFileSync(path.join(__dirname, 'shape', 'arc.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: shape/arc.json');
}

function generatePieTests() {
  const testCases = [];

  // Basic pie
  {
    const data = [1, 2, 3, 4];
    const pie = d3.pie();
    const arcs = pie(data);

    testCases.push({
      name: "basic",
      data,
      arcs: arcs.map(a => ({
        value: a.value,
        startAngle: a.startAngle,
        endAngle: a.endAngle,
        padAngle: a.padAngle,
        index: a.index
      }))
    });
  }

  // With padding
  {
    const data = [1, 2, 3, 4];
    const pie = d3.pie().padAngle(0.05);
    const arcs = pie(data);

    testCases.push({
      name: "with_padding",
      data,
      padAngle: 0.05,
      arcs: arcs.map(a => ({
        value: a.value,
        startAngle: a.startAngle,
        endAngle: a.endAngle,
        padAngle: a.padAngle
      }))
    });
  }

  // Custom start/end angles
  {
    const data = [1, 2, 3];
    const pie = d3.pie()
      .startAngle(-Math.PI / 2)
      .endAngle(Math.PI / 2);
    const arcs = pie(data);

    testCases.push({
      name: "half_pie",
      data,
      startAngle: -Math.PI / 2,
      endAngle: Math.PI / 2,
      arcs: arcs.map(a => ({
        value: a.value,
        startAngle: a.startAngle,
        endAngle: a.endAngle
      }))
    });
  }

  // Sorted
  {
    const data = [3, 1, 4, 1, 5];
    const pie = d3.pie().sort((a, b) => b - a);
    const arcs = pie(data);

    testCases.push({
      name: "sorted_descending",
      data,
      arcs: arcs.map(a => ({
        value: a.value,
        index: a.index,
        startAngle: a.startAngle,
        endAngle: a.endAngle
      }))
    });
  }

  const golden = createGoldenFile("d3-shape", "pie", testCases);
  fs.writeFileSync(path.join(__dirname, 'shape', 'pie.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: shape/pie.json');
}

function generateLineTests() {
  const testCases = [];

  // Linear curve
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30], [40, 15]];
    const line = d3.line();
    testCases.push({
      name: "linear",
      data,
      curve: "linear",
      path: line(data)
    });
  }

  // Step curve
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30]];
    const line = d3.line().curve(d3.curveStep);
    testCases.push({
      name: "step",
      data,
      curve: "step",
      path: line(data)
    });
  }

  // Basis curve
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30], [40, 15]];
    const line = d3.line().curve(d3.curveBasis);
    testCases.push({
      name: "basis",
      data,
      curve: "basis",
      path: line(data)
    });
  }

  // Cardinal curve
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30], [40, 15]];
    const line = d3.line().curve(d3.curveCardinal);
    testCases.push({
      name: "cardinal",
      data,
      curve: "cardinal",
      path: line(data)
    });
  }

  // Catmull-Rom curve
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30], [40, 15]];
    const line = d3.line().curve(d3.curveCatmullRom);
    testCases.push({
      name: "catmull_rom",
      data,
      curve: "catmullRom",
      path: line(data)
    });
  }

  // Monotone X
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30], [40, 15]];
    const line = d3.line().curve(d3.curveMonotoneX);
    testCases.push({
      name: "monotone_x",
      data,
      curve: "monotoneX",
      path: line(data)
    });
  }

  // Natural
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30], [40, 15]];
    const line = d3.line().curve(d3.curveNatural);
    testCases.push({
      name: "natural",
      data,
      curve: "natural",
      path: line(data)
    });
  }

  const golden = createGoldenFile("d3-shape", "line", testCases);
  fs.writeFileSync(path.join(__dirname, 'shape', 'line.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: shape/line.json');
}

function generateSymbolTests() {
  const testCases = [];

  const symbolTypes = [
    { name: 'circle', type: d3.symbolCircle },
    { name: 'cross', type: d3.symbolCross },
    { name: 'diamond', type: d3.symbolDiamond },
    { name: 'square', type: d3.symbolSquare },
    { name: 'star', type: d3.symbolStar },
    { name: 'triangle', type: d3.symbolTriangle },
    { name: 'wye', type: d3.symbolWye }
  ];

  for (const { name, type } of symbolTypes) {
    const symbol = d3.symbol().type(type).size(64);
    testCases.push({
      name,
      size: 64,
      path: symbol()
    });
  }

  // Different sizes
  {
    const sizes = [16, 64, 256];
    for (const size of sizes) {
      const symbol = d3.symbol().type(d3.symbolCircle).size(size);
      testCases.push({
        name: `circle_size_${size}`,
        size,
        path: symbol()
      });
    }
  }

  const golden = createGoldenFile("d3-shape", "symbol", testCases);
  fs.writeFileSync(path.join(__dirname, 'shape', 'symbol.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: shape/symbol.json');
}

function generateStackTests() {
  const testCases = [];

  // Basic stack
  {
    const data = [
      { month: 'Jan', apples: 10, oranges: 20, bananas: 15 },
      { month: 'Feb', apples: 15, oranges: 25, bananas: 10 },
      { month: 'Mar', apples: 20, oranges: 30, bananas: 20 }
    ];

    const stack = d3.stack().keys(['apples', 'oranges', 'bananas']);
    const stacked = stack(data);

    testCases.push({
      name: "basic",
      data,
      keys: ['apples', 'oranges', 'bananas'],
      result: stacked.map(series => ({
        key: series.key,
        values: series.map(d => [d[0], d[1]])
      }))
    });
  }

  // With offset expand (normalize to 100%)
  {
    const data = [
      { a: 10, b: 20, c: 30 },
      { a: 20, b: 30, c: 40 },
      { a: 30, b: 40, c: 50 }
    ];

    const stack = d3.stack()
      .keys(['a', 'b', 'c'])
      .offset(d3.stackOffsetExpand);
    const stacked = stack(data);

    testCases.push({
      name: "offset_expand",
      data,
      keys: ['a', 'b', 'c'],
      offset: 'expand',
      result: stacked.map(series => ({
        key: series.key,
        values: series.map(d => [d[0], d[1]])
      }))
    });
  }

  // Diverging offset
  {
    const data = [
      { pos: 10, neg: -5 },
      { pos: 20, neg: -10 },
      { pos: 15, neg: -8 }
    ];

    const stack = d3.stack()
      .keys(['pos', 'neg'])
      .offset(d3.stackOffsetDiverging);
    const stacked = stack(data);

    testCases.push({
      name: "offset_diverging",
      data,
      keys: ['pos', 'neg'],
      offset: 'diverging',
      result: stacked.map(series => ({
        key: series.key,
        values: series.map(d => [d[0], d[1]])
      }))
    });
  }

  const golden = createGoldenFile("d3-shape", "stack", testCases);
  fs.writeFileSync(path.join(__dirname, 'shape', 'stack.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: shape/stack.json');
}

// ============================================================================
// EASE GENERATORS
// ============================================================================

function generateEaseTests() {
  const testCases = [];
  const ts = [0, 0.25, 0.5, 0.75, 1];

  // Linear
  testCases.push({
    name: "linear",
    inputs: ts,
    outputs: ts.map(t => d3.easeLinear(t))
  });

  // Quad
  testCases.push({
    name: "quad_in",
    inputs: ts,
    outputs: ts.map(t => d3.easeQuadIn(t))
  });
  testCases.push({
    name: "quad_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeQuadOut(t))
  });
  testCases.push({
    name: "quad_in_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeQuadInOut(t))
  });

  // Cubic
  testCases.push({
    name: "cubic_in",
    inputs: ts,
    outputs: ts.map(t => d3.easeCubicIn(t))
  });
  testCases.push({
    name: "cubic_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeCubicOut(t))
  });
  testCases.push({
    name: "cubic_in_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeCubicInOut(t))
  });

  // Sin
  testCases.push({
    name: "sin_in",
    inputs: ts,
    outputs: ts.map(t => d3.easeSinIn(t))
  });
  testCases.push({
    name: "sin_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeSinOut(t))
  });
  testCases.push({
    name: "sin_in_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeSinInOut(t))
  });

  // Exp
  testCases.push({
    name: "exp_in",
    inputs: ts,
    outputs: ts.map(t => d3.easeExpIn(t))
  });
  testCases.push({
    name: "exp_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeExpOut(t))
  });
  testCases.push({
    name: "exp_in_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeExpInOut(t))
  });

  // Circle
  testCases.push({
    name: "circle_in",
    inputs: ts,
    outputs: ts.map(t => d3.easeCircleIn(t))
  });
  testCases.push({
    name: "circle_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeCircleOut(t))
  });
  testCases.push({
    name: "circle_in_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeCircleInOut(t))
  });

  // Elastic
  testCases.push({
    name: "elastic_in",
    inputs: ts,
    outputs: ts.map(t => d3.easeElasticIn(t))
  });
  testCases.push({
    name: "elastic_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeElasticOut(t))
  });
  testCases.push({
    name: "elastic_in_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeElasticInOut(t))
  });

  // Back
  testCases.push({
    name: "back_in",
    inputs: ts,
    outputs: ts.map(t => d3.easeBackIn(t))
  });
  testCases.push({
    name: "back_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeBackOut(t))
  });
  testCases.push({
    name: "back_in_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeBackInOut(t))
  });

  // Bounce
  testCases.push({
    name: "bounce_in",
    inputs: ts,
    outputs: ts.map(t => d3.easeBounceIn(t))
  });
  testCases.push({
    name: "bounce_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeBounceOut(t))
  });
  testCases.push({
    name: "bounce_in_out",
    inputs: ts,
    outputs: ts.map(t => d3.easeBounceInOut(t))
  });

  // Poly with different exponents
  testCases.push({
    name: "poly_in_2",
    exponent: 2,
    inputs: ts,
    outputs: ts.map(t => d3.easePolyIn.exponent(2)(t))
  });
  testCases.push({
    name: "poly_in_3",
    exponent: 3,
    inputs: ts,
    outputs: ts.map(t => d3.easePolyIn.exponent(3)(t))
  });
  testCases.push({
    name: "poly_in_4",
    exponent: 4,
    inputs: ts,
    outputs: ts.map(t => d3.easePolyIn.exponent(4)(t))
  });

  const golden = createGoldenFile("d3-ease", "ease", testCases);
  fs.mkdirSync(path.join(__dirname, 'ease'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'ease', 'ease.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: ease/ease.json');
}

// ============================================================================
// CONTOUR GENERATORS
// ============================================================================

function generateContourTests() {
  const testCases = [];

  // Basic contour density
  {
    const points = [
      [0, 0], [1, 1], [2, 0], [1, 2], [0, 2], [2, 2],
      [0.5, 0.5], [1.5, 0.5], [0.5, 1.5], [1.5, 1.5]
    ];

    const density = d3.contourDensity()
      .x(d => d[0])
      .y(d => d[1])
      .size([3, 3])
      .bandwidth(0.5)
      .thresholds(5);

    const contours = density(points);

    testCases.push({
      name: "basic_density",
      points,
      size: [3, 3],
      bandwidth: 0.5,
      threshold_count: 5,
      contour_count: contours.length,
      values: contours.map(c => c.value)
    });
  }

  // Contours from grid data
  {
    // Simple 3x3 grid with peak in center
    const values = [
      0, 0, 0,
      0, 1, 0,
      0, 0, 0
    ];

    const contours = d3.contours()
      .size([3, 3])
      .thresholds([0.25, 0.5, 0.75])
      (values);

    testCases.push({
      name: "grid_contours",
      values,
      size: [3, 3],
      thresholds: [0.25, 0.5, 0.75],
      contour_count: contours.length,
      contours: contours.map(c => ({
        value: c.value,
        coordinates_count: c.coordinates.length
      }))
    });
  }

  // Larger grid with gradients
  {
    const n = 10;
    const values = [];
    for (let j = 0; j < n; j++) {
      for (let i = 0; i < n; i++) {
        // Distance from center creates radial gradient
        const dx = i - n/2;
        const dy = j - n/2;
        values.push(Math.exp(-(dx*dx + dy*dy) / 10));
      }
    }

    const contours = d3.contours()
      .size([n, n])
      .thresholds(d3.range(0.1, 1, 0.2))
      (values);

    testCases.push({
      name: "radial_gradient",
      size: [n, n],
      thresholds: d3.range(0.1, 1, 0.2),
      contour_count: contours.length,
      values: contours.map(c => c.value)
    });
  }

  const golden = createGoldenFile("d3-contour", "contour", testCases);
  fs.mkdirSync(path.join(__dirname, 'contour'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'contour', 'contour.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: contour/contour.json');
}

// ============================================================================
// DELAUNAY GENERATORS
// ============================================================================

function generateDelaunayTests() {
  const testCases = [];

  // Basic Delaunay triangulation
  {
    const points = [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]];
    const delaunay = d3.Delaunay.from(points);

    testCases.push({
      name: "basic_triangulation",
      points,
      triangles: Array.from(delaunay.triangles),
      hull: Array.from(delaunay.hull)
    });
  }

  // Voronoi diagram
  {
    const points = [[0, 0], [1, 0], [0, 1], [1, 1]];
    const delaunay = d3.Delaunay.from(points);
    const voronoi = delaunay.voronoi([0, 0, 1, 1]);

    testCases.push({
      name: "voronoi_basic",
      points,
      bounds: [0, 0, 1, 1],
      cell_polygons: points.map((_, i) => voronoi.cellPolygon(i))
    });
  }

  // Find nearest neighbor
  {
    const points = [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]];
    const delaunay = d3.Delaunay.from(points);

    const queries = [
      [0.3, 0.3],
      [0.9, 0.1],
      [0.5, 0.5],
      [0.7, 0.8]
    ];

    testCases.push({
      name: "find_nearest",
      points,
      queries: queries.map(q => ({
        query: q,
        nearest_index: delaunay.find(q[0], q[1])
      }))
    });
  }

  // Neighbors
  {
    const points = [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]];
    const delaunay = d3.Delaunay.from(points);

    testCases.push({
      name: "neighbors",
      points,
      neighbors: points.map((_, i) => Array.from(delaunay.neighbors(i)))
    });
  }

  const golden = createGoldenFile("d3-delaunay", "delaunay", testCases);
  fs.mkdirSync(path.join(__dirname, 'delaunay'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'delaunay', 'delaunay.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: delaunay/delaunay.json');
}

// ============================================================================
// AREA SHAPE GENERATORS
// ============================================================================

function generateAreaTests() {
  const testCases = [];

  // Basic area
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30], [40, 15]];
    const area = d3.area()
      .x(d => d[0])
      .y0(0)
      .y1(d => d[1]);

    testCases.push({
      name: "basic",
      data,
      path: area(data)
    });
  }

  // Area with baseline
  {
    const data = [[0, 10], [10, 30], [20, 20], [30, 40], [40, 25]];
    const area = d3.area()
      .x(d => d[0])
      .y0(5)
      .y1(d => d[1]);

    testCases.push({
      name: "with_baseline",
      data,
      baseline: 5,
      path: area(data)
    });
  }

  // Stacked area (y0 and y1 both variable)
  {
    const data = [
      { x: 0, y0: 0, y1: 10 },
      { x: 10, y0: 5, y1: 25 },
      { x: 20, y0: 10, y1: 20 },
      { x: 30, y0: 8, y1: 35 },
      { x: 40, y0: 3, y1: 20 }
    ];
    const area = d3.area()
      .x(d => d.x)
      .y0(d => d.y0)
      .y1(d => d.y1);

    testCases.push({
      name: "stacked",
      data,
      path: area(data)
    });
  }

  // With step curve
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30]];
    const area = d3.area()
      .x(d => d[0])
      .y0(0)
      .y1(d => d[1])
      .curve(d3.curveStep);

    testCases.push({
      name: "step_curve",
      data,
      curve: "step",
      path: area(data)
    });
  }

  // With monotone curve
  {
    const data = [[0, 0], [10, 20], [20, 10], [30, 30], [40, 15]];
    const area = d3.area()
      .x(d => d[0])
      .y0(0)
      .y1(d => d[1])
      .curve(d3.curveMonotoneX);

    testCases.push({
      name: "monotone_x",
      data,
      curve: "monotoneX",
      path: area(data)
    });
  }

  const golden = createGoldenFile("d3-shape", "area", testCases);
  fs.writeFileSync(path.join(__dirname, 'shape', 'area.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: shape/area.json');
}

// ============================================================================
// INTERPOLATE STRING GENERATORS
// ============================================================================

function generateInterpolateStringTests() {
  const testCases = [];

  // Basic string with numbers
  {
    const interp = d3.interpolateString("10px", "20px");
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "basic_px",
      a: "10px",
      b: "20px",
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Multiple numbers
  {
    const interp = d3.interpolateString("translate(0, 0)", "translate(100, 50)");
    const ts = [0, 0.5, 1];
    testCases.push({
      name: "translate",
      a: "translate(0, 0)",
      b: "translate(100, 50)",
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Color in string
  {
    const interp = d3.interpolateString("1px solid #ff0000", "5px solid #0000ff");
    const ts = [0, 0.5, 1];
    testCases.push({
      name: "border",
      a: "1px solid #ff0000",
      b: "5px solid #0000ff",
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Decimal numbers
  {
    const interp = d3.interpolateString("0.5em", "2.5em");
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "decimal",
      a: "0.5em",
      b: "2.5em",
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  // Transform with rotation
  {
    const interp = d3.interpolateString("rotate(0deg)", "rotate(180deg)");
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "rotate",
      a: "rotate(0deg)",
      b: "rotate(180deg)",
      inputs: ts,
      outputs: ts.map(t => interp(t))
    });
  }

  const golden = createGoldenFile("d3-interpolate", "interpolateString", testCases);
  fs.writeFileSync(path.join(__dirname, 'interpolate', 'string.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: interpolate/string.json');
}

// ============================================================================
// ARRAY TRANSFORM GENERATORS
// ============================================================================

function generateArrayTransformTests() {
  const testCases = [];

  // shuffle (note: random, so we just test length preservation)
  {
    const arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const shuffled = d3.shuffle(arr.slice());
    testCases.push({
      name: "shuffle",
      original: arr,
      shuffled_length: shuffled.length,
      shuffled_sorted: shuffled.slice().sort((a, b) => a - b)
    });
  }

  // reverse
  {
    const arr = [1, 2, 3, 4, 5];
    testCases.push({
      name: "reverse",
      original: arr,
      reversed: d3.reverse(arr)
    });
  }

  // sort
  {
    const arr = [5, 2, 8, 1, 9, 3];
    testCases.push({
      name: "sort_ascending",
      original: arr,
      sorted: d3.sort(arr)
    });
  }

  // sort descending
  {
    const arr = [5, 2, 8, 1, 9, 3];
    testCases.push({
      name: "sort_descending",
      original: arr,
      sorted: d3.sort(arr, d3.descending)
    });
  }

  // permute
  {
    const arr = ['a', 'b', 'c', 'd', 'e'];
    const keys = [4, 0, 2];
    testCases.push({
      name: "permute",
      original: arr,
      keys,
      permuted: d3.permute(arr, keys)
    });
  }

  // zip
  {
    const a = [1, 2, 3];
    const b = [4, 5, 6];
    const c = [7, 8, 9];
    testCases.push({
      name: "zip",
      arrays: [a, b, c],
      zipped: d3.zip(a, b, c)
    });
  }

  // transpose
  {
    const matrix = [[1, 2, 3], [4, 5, 6]];
    testCases.push({
      name: "transpose",
      original: matrix,
      transposed: d3.transpose(matrix)
    });
  }

  // cross
  {
    const a = [1, 2];
    const b = ['x', 'y', 'z'];
    testCases.push({
      name: "cross",
      a,
      b,
      crossed: d3.cross(a, b)
    });
  }

  // pairs
  {
    const arr = [1, 2, 3, 4, 5];
    testCases.push({
      name: "pairs",
      original: arr,
      pairs: d3.pairs(arr)
    });
  }

  // range
  {
    testCases.push({
      name: "range_basic",
      stop: 5,
      result: d3.range(5)
    });
    testCases.push({
      name: "range_start_stop",
      start: 1,
      stop: 5,
      result: d3.range(1, 5)
    });
    testCases.push({
      name: "range_step",
      start: 0,
      stop: 1,
      step: 0.2,
      result: d3.range(0, 1, 0.2)
    });
  }

  const golden = createGoldenFile("d3-array", "transform", testCases);
  fs.writeFileSync(path.join(__dirname, 'array', 'transform.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: array/transform.json');
}

// ============================================================================
// QUADTREE GENERATORS
// ============================================================================

function generateQuadtreeTests() {
  const testCases = [];

  // Basic quadtree with add
  {
    const tree = d3.quadtree();
    tree.add([0, 0]);
    tree.add([1, 0]);
    tree.add([0, 1]);
    tree.add([1, 1]);
    tree.add([0.5, 0.5]);

    testCases.push({
      name: "basic_add",
      points: [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]],
      size: tree.size(),
      extent: tree.extent()
    });
  }

  // Find nearest neighbor
  {
    const tree = d3.quadtree()
      .addAll([[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]]);

    const queries = [
      { x: 0.3, y: 0.3 },
      { x: 0.9, y: 0.1 },
      { x: 0.5, y: 0.5 },
      { x: 2, y: 2 }
    ];

    testCases.push({
      name: "find",
      points: [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]],
      queries: queries.map(q => ({
        x: q.x,
        y: q.y,
        result: tree.find(q.x, q.y)
      }))
    });
  }

  // Find with radius
  {
    const tree = d3.quadtree()
      .addAll([[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]]);

    testCases.push({
      name: "find_with_radius",
      points: [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]],
      queries: [
        { x: 0.5, y: 0.5, radius: 0.1, result: tree.find(0.5, 0.5, 0.1) },
        { x: 0.5, y: 0.5, radius: 0.5, result: tree.find(0.5, 0.5, 0.5) },
        { x: 0.5, y: 0.5, radius: 1.0, result: tree.find(0.5, 0.5, 1.0) },
        { x: 10, y: 10, radius: 0.5, result: tree.find(10, 10, 0.5) }
      ]
    });
  }

  // Remove points
  // Note: D3.js remove() requires the exact same object reference that was added.
  // Our Rust implementation removes by coordinates, which is more practical.
  // This test uses find() to get the reference first, which is what users should do in D3.
  {
    const tree = d3.quadtree()
      .addAll([[0, 0], [1, 0], [0, 1], [1, 1]]);

    const sizeBefore = tree.size();
    // Find the point first to get the reference
    const pointToRemove = tree.find(1, 0, 0.001);
    if (pointToRemove) {
      tree.remove(pointToRemove);
    }
    const sizeAfter = tree.size();

    testCases.push({
      name: "remove",
      points: [[0, 0], [1, 0], [0, 1], [1, 1]],
      remove: [1, 0],
      size_before: sizeBefore,
      size_after: sizeAfter
    });
  }

  // Extent
  {
    const tree = d3.quadtree()
      .addAll([[0, 0], [3, 2], [-1, 5], [7, -3]]);

    testCases.push({
      name: "extent",
      points: [[0, 0], [3, 2], [-1, 5], [7, -3]],
      extent: tree.extent(),
      size: tree.size()
    });
  }

  // Visit traversal
  {
    const tree = d3.quadtree()
      .addAll([[0, 0], [1, 0], [0, 1], [1, 1]]);

    const visited = [];
    tree.visit((node, x0, y0, x1, y1) => {
      visited.push({
        x0, y0, x1, y1,
        is_leaf: !node.length
      });
      return false; // continue visiting
    });

    testCases.push({
      name: "visit",
      points: [[0, 0], [1, 0], [0, 1], [1, 1]],
      visited_count: visited.length,
      leaf_count: visited.filter(v => v.is_leaf).length
    });
  }

  // Data extraction
  {
    const points = [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]];
    const tree = d3.quadtree().addAll(points);

    testCases.push({
      name: "data",
      points,
      data: tree.data()
    });
  }

  // Coincident points
  {
    const tree = d3.quadtree();
    tree.add([5, 5]);
    tree.add([5, 5]);
    tree.add([5, 5]);

    testCases.push({
      name: "coincident",
      points: [[5, 5], [5, 5], [5, 5]],
      size: tree.size(),
      data: tree.data()
    });
  }

  // Large dataset for performance reference
  {
    const points = [];
    for (let i = 0; i < 100; i++) {
      // Use golden ratio for even distribution
      const x = (i * 0.618033988749895) % 1 * 100;
      const y = (i * 0.381966011250105) % 1 * 100;
      points.push([x, y]);
    }

    const tree = d3.quadtree().addAll(points);

    testCases.push({
      name: "large_dataset",
      point_count: 100,
      size: tree.size(),
      extent: tree.extent(),
      // Find some specific points
      find_50_50: tree.find(50, 50),
      find_0_0: tree.find(0, 0),
      find_100_100: tree.find(100, 100)
    });
  }

  const golden = createGoldenFile("d3-quadtree", "quadtree", testCases);
  fs.writeFileSync(path.join(__dirname, 'quadtree', 'quadtree.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: quadtree/quadtree.json');
}

// ============================================================================
// BRUSH GENERATORS
// ============================================================================

function generateBrushTests() {
  const testCases = [];

  // Note: d3-brush is primarily a DOM-based module for interactive brushing.
  // Since golden tests are for data transformations, we test the selection/extent logic.

  // Brush extent calculation
  {
    testCases.push({
      name: "brush_extent_2d",
      description: "2D brush selection bounds",
      selection: [[10, 20], [100, 80]],
      expected_x0: 10,
      expected_y0: 20,
      expected_x1: 100,
      expected_y1: 80,
      width: 90,
      height: 60
    });
  }

  // Brush X only
  {
    testCases.push({
      name: "brush_extent_x",
      description: "1D horizontal brush",
      selection: [10, 100],
      expected_x0: 10,
      expected_x1: 100,
      width: 90
    });
  }

  // Brush Y only
  {
    testCases.push({
      name: "brush_extent_y",
      description: "1D vertical brush",
      selection: [20, 80],
      expected_y0: 20,
      expected_y1: 80,
      height: 60
    });
  }

  // Empty selection
  {
    testCases.push({
      name: "brush_empty",
      description: "No selection (null)",
      selection: null
    });
  }

  // Clamped selection
  {
    testCases.push({
      name: "brush_clamped",
      description: "Selection clamped to extent",
      extent: [[0, 0], [200, 100]],
      input_selection: [[-10, -10], [250, 150]],
      clamped_selection: [[0, 0], [200, 100]]
    });
  }

  const golden = createGoldenFile("d3-brush", "brush", testCases);
  fs.mkdirSync(path.join(__dirname, 'brush'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'brush', 'brush.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: brush/brush.json');
}

// ============================================================================
// ZOOM GENERATORS
// ============================================================================

function generateZoomTests() {
  const testCases = [];

  // ZoomTransform identity
  {
    const identity = d3.zoomIdentity;
    testCases.push({
      name: "identity",
      k: identity.k,
      x: identity.x,
      y: identity.y,
      string: identity.toString()
    });
  }

  // Translate
  {
    const transform = d3.zoomIdentity.translate(100, 50);
    testCases.push({
      name: "translate",
      k: transform.k,
      x: transform.x,
      y: transform.y,
      string: transform.toString()
    });
  }

  // Scale
  {
    const transform = d3.zoomIdentity.scale(2);
    testCases.push({
      name: "scale",
      k: transform.k,
      x: transform.x,
      y: transform.y,
      string: transform.toString()
    });
  }

  // Combined translate and scale
  {
    const transform = d3.zoomIdentity.translate(100, 50).scale(2);
    testCases.push({
      name: "translate_then_scale",
      k: transform.k,
      x: transform.x,
      y: transform.y,
      string: transform.toString()
    });
  }

  // Apply transform to point
  {
    const transform = d3.zoomIdentity.translate(100, 50).scale(2);
    const point = [10, 20];
    testCases.push({
      name: "apply_to_point",
      transform: { k: transform.k, x: transform.x, y: transform.y },
      input_point: point,
      output_x: transform.apply(point)[0],
      output_y: transform.apply(point)[1]
    });
  }

  // Invert point
  {
    const transform = d3.zoomIdentity.translate(100, 50).scale(2);
    const point = [120, 90];
    testCases.push({
      name: "invert_point",
      transform: { k: transform.k, x: transform.x, y: transform.y },
      input_point: point,
      output_x: transform.invert(point)[0],
      output_y: transform.invert(point)[1]
    });
  }

  // Rescale X
  {
    const transform = d3.zoomIdentity.translate(100, 0).scale(2);
    const xScale = d3.scaleLinear().domain([0, 100]).range([0, 500]);
    const rescaledX = transform.rescaleX(xScale);
    testCases.push({
      name: "rescale_x",
      transform: { k: transform.k, x: transform.x, y: transform.y },
      original_domain: [0, 100],
      original_range: [0, 500],
      rescaled_domain: rescaledX.domain()
    });
  }

  // Rescale Y
  {
    const transform = d3.zoomIdentity.translate(0, 50).scale(2);
    const yScale = d3.scaleLinear().domain([0, 100]).range([0, 500]);
    const rescaledY = transform.rescaleY(yScale);
    testCases.push({
      name: "rescale_y",
      transform: { k: transform.k, x: transform.x, y: transform.y },
      original_domain: [0, 100],
      original_range: [0, 500],
      rescaled_domain: rescaledY.domain()
    });
  }

  // Interpolate zoom
  {
    const start = [0, 0, 100]; // [x, y, width]
    const end = [100, 50, 50]; // zoom in and translate
    const interp = d3.interpolateZoom(start, end);
    const ts = [0, 0.25, 0.5, 0.75, 1];
    testCases.push({
      name: "interpolate_zoom",
      start,
      end,
      inputs: ts,
      outputs: ts.map(t => interp(t)),
      duration: interp.duration
    });
  }

  const golden = createGoldenFile("d3-zoom", "zoom", testCases);
  fs.mkdirSync(path.join(__dirname, 'zoom'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'zoom', 'zoom.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: zoom/zoom.json');
}

// ============================================================================
// FORMAT GENERATORS
// ============================================================================

function generateFormatTests() {
  const testCases = [];

  // Basic number formatting
  {
    const values = [0, 1, 10, 100, 1000, 10000, 100000, 1000000];
    testCases.push({
      name: "default",
      values,
      formatted: values.map(v => d3.format("")(v))
    });
  }

  // Fixed precision
  {
    const values = [0.123456, 1.23456, 12.3456, 123.456];
    testCases.push({
      name: "fixed_2",
      specifier: ".2f",
      values,
      formatted: values.map(v => d3.format(".2f")(v))
    });
  }

  // Thousands separator
  {
    const values = [1000, 10000, 100000, 1000000];
    testCases.push({
      name: "thousands",
      specifier: ",",
      values,
      formatted: values.map(v => d3.format(",")(v))
    });
  }

  // SI prefix
  {
    const values = [0.001, 0.01, 0.1, 1, 10, 100, 1000, 1000000, 1000000000];
    testCases.push({
      name: "si_prefix",
      specifier: ".2s",
      values,
      formatted: values.map(v => d3.format(".2s")(v))
    });
  }

  // Percentage
  {
    const values = [0, 0.1, 0.25, 0.5, 0.75, 1, 1.5];
    testCases.push({
      name: "percentage",
      specifier: ".0%",
      values,
      formatted: values.map(v => d3.format(".0%")(v))
    });
  }

  // Percentage with decimals
  {
    const values = [0.123, 0.456, 0.789];
    testCases.push({
      name: "percentage_decimal",
      specifier: ".1%",
      values,
      formatted: values.map(v => d3.format(".1%")(v))
    });
  }

  // Exponential notation
  {
    const values = [0.00123, 0.0123, 0.123, 1.23, 12.3, 123, 1230, 12300];
    testCases.push({
      name: "exponential",
      specifier: ".2e",
      values,
      formatted: values.map(v => d3.format(".2e")(v))
    });
  }

  // Binary
  {
    const values = [0, 1, 2, 4, 8, 16, 255];
    testCases.push({
      name: "binary",
      specifier: "b",
      values,
      formatted: values.map(v => d3.format("b")(v))
    });
  }

  // Hexadecimal
  {
    const values = [0, 10, 16, 255, 4096];
    testCases.push({
      name: "hexadecimal",
      specifier: "x",
      values,
      formatted: values.map(v => d3.format("x")(v))
    });
  }

  // Sign always
  {
    const values = [-100, -10, 0, 10, 100];
    testCases.push({
      name: "sign_always",
      specifier: "+",
      values,
      formatted: values.map(v => d3.format("+")(v))
    });
  }

  // Padding with zeros
  {
    const values = [1, 10, 100, 1000];
    testCases.push({
      name: "zero_padding",
      specifier: "06d",
      values,
      formatted: values.map(v => d3.format("06d")(v))
    });
  }

  // Currency-like
  {
    const values = [1234.56, 12345.67, 123456.78];
    testCases.push({
      name: "currency",
      specifier: "$,.2f",
      values,
      formatted: values.map(v => d3.format("$,.2f")(v))
    });
  }

  // FormatSpecifier parsing
  {
    const specifiers = [".2f", ",", ".2s", "+.2f", "06d"];
    testCases.push({
      name: "specifier_parsing",
      specifiers: specifiers.map(s => {
        const spec = d3.formatSpecifier(s);
        return {
          input: s,
          fill: spec.fill,
          align: spec.align,
          sign: spec.sign,
          symbol: spec.symbol,
          zero: spec.zero,
          width: spec.width,
          comma: spec.comma,
          precision: spec.precision,
          trim: spec.trim,
          type: spec.type
        };
      })
    });
  }

  const golden = createGoldenFile("d3-format", "format", testCases);
  fs.mkdirSync(path.join(__dirname, 'format'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'format', 'format.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: format/format.json');
}

// ============================================================================
// TIME GENERATORS
// ============================================================================

function generateTimeTests() {
  const testCases = [];

  // Time intervals
  {
    const date = new Date(2024, 0, 15, 12, 30, 45, 500); // Jan 15, 2024, 12:30:45.500

    testCases.push({
      name: "floor_intervals",
      input: date.toISOString(),
      second: d3.timeSecond.floor(date).toISOString(),
      minute: d3.timeMinute.floor(date).toISOString(),
      hour: d3.timeHour.floor(date).toISOString(),
      day: d3.timeDay.floor(date).toISOString(),
      week: d3.timeWeek.floor(date).toISOString(),
      month: d3.timeMonth.floor(date).toISOString(),
      year: d3.timeYear.floor(date).toISOString()
    });
  }

  // Ceil intervals
  {
    const date = new Date(2024, 0, 15, 12, 30, 45, 500);

    testCases.push({
      name: "ceil_intervals",
      input: date.toISOString(),
      second: d3.timeSecond.ceil(date).toISOString(),
      minute: d3.timeMinute.ceil(date).toISOString(),
      hour: d3.timeHour.ceil(date).toISOString(),
      day: d3.timeDay.ceil(date).toISOString(),
      month: d3.timeMonth.ceil(date).toISOString()
    });
  }

  // Range generation
  {
    const start = new Date(2024, 0, 1); // Jan 1, 2024
    const end = new Date(2024, 0, 8);   // Jan 8, 2024

    testCases.push({
      name: "range_days",
      start: start.toISOString(),
      end: end.toISOString(),
      days: d3.timeDays(start, end).map(d => d.toISOString())
    });
  }

  // Hours range
  {
    const start = new Date(2024, 0, 1, 0, 0, 0);
    const end = new Date(2024, 0, 1, 6, 0, 0);

    testCases.push({
      name: "range_hours",
      start: start.toISOString(),
      end: end.toISOString(),
      hours: d3.timeHours(start, end).map(d => d.toISOString())
    });
  }

  // Month range
  {
    const start = new Date(2024, 0, 1);
    const end = new Date(2024, 6, 1);

    testCases.push({
      name: "range_months",
      start: start.toISOString(),
      end: end.toISOString(),
      months: d3.timeMonths(start, end).map(d => d.toISOString())
    });
  }

  // Count between dates
  {
    const start = new Date(2024, 0, 1);
    const end = new Date(2024, 0, 15);

    testCases.push({
      name: "count_days",
      start: start.toISOString(),
      end: end.toISOString(),
      count: d3.timeDay.count(start, end)
    });
  }

  // Offset
  {
    const date = new Date(2024, 0, 15);

    testCases.push({
      name: "offset",
      input: date.toISOString(),
      plus_5_days: d3.timeDay.offset(date, 5).toISOString(),
      minus_3_days: d3.timeDay.offset(date, -3).toISOString(),
      plus_2_months: d3.timeMonth.offset(date, 2).toISOString(),
      plus_1_year: d3.timeYear.offset(date, 1).toISOString()
    });
  }

  // Time formatting
  {
    const date = new Date(2024, 5, 15, 14, 30, 45); // June 15, 2024, 14:30:45

    testCases.push({
      name: "format",
      input: date.toISOString(),
      formats: {
        "%Y-%m-%d": d3.timeFormat("%Y-%m-%d")(date),
        "%B %d, %Y": d3.timeFormat("%B %d, %Y")(date),
        "%H:%M:%S": d3.timeFormat("%H:%M:%S")(date),
        "%I:%M %p": d3.timeFormat("%I:%M %p")(date),
        "%a %b %d": d3.timeFormat("%a %b %d")(date),
        "%x": d3.timeFormat("%x")(date),
        "%X": d3.timeFormat("%X")(date)
      }
    });
  }

  // Time parsing
  {
    const formats = [
      { format: "%Y-%m-%d", input: "2024-06-15" },
      { format: "%m/%d/%Y", input: "06/15/2024" },
      { format: "%B %d, %Y", input: "June 15, 2024" }
    ];

    testCases.push({
      name: "parse",
      results: formats.map(f => {
        const parsed = d3.timeParse(f.format)(f.input);
        return {
          format: f.format,
          input: f.input,
          output: parsed ? parsed.toISOString() : null
        };
      })
    });
  }

  // Time ticks
  {
    const start = new Date(2024, 0, 1);
    const end = new Date(2024, 11, 31);

    testCases.push({
      name: "ticks",
      start: start.toISOString(),
      end: end.toISOString(),
      ticks_10: d3.timeTicks(start, end, 10).map(d => d.toISOString())
    });
  }

  const golden = createGoldenFile("d3-time", "time", testCases);
  fs.mkdirSync(path.join(__dirname, 'time'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'time', 'time.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: time/time.json');
}

// ============================================================================
// AXIS GENERATORS
// ============================================================================

function generateAxisTests() {
  const testCases = [];

  // Note: d3-axis generates SVG elements. For golden tests we verify
  // the tick values and positions that would be generated.

  // Linear scale ticks for bottom axis
  {
    const scale = d3.scaleLinear().domain([0, 100]).range([0, 500]);
    const ticks = scale.ticks(10);

    testCases.push({
      name: "linear_bottom",
      orientation: "bottom",
      domain: [0, 100],
      range: [0, 500],
      tick_count: 10,
      ticks,
      tick_positions: ticks.map(t => scale(t))
    });
  }

  // Linear scale ticks for left axis
  {
    const scale = d3.scaleLinear().domain([0, 100]).range([500, 0]);
    const ticks = scale.ticks(10);

    testCases.push({
      name: "linear_left",
      orientation: "left",
      domain: [0, 100],
      range: [500, 0],
      tick_count: 10,
      ticks,
      tick_positions: ticks.map(t => scale(t))
    });
  }

  // Log scale ticks
  {
    const scale = d3.scaleLog().domain([1, 1000]).range([0, 500]);
    const ticks = scale.ticks();

    testCases.push({
      name: "log_scale",
      orientation: "bottom",
      domain: [1, 1000],
      range: [0, 500],
      ticks,
      tick_positions: ticks.map(t => scale(t))
    });
  }

  // Time scale ticks
  {
    const start = new Date(2024, 0, 1);
    const end = new Date(2024, 11, 31);
    const scale = d3.scaleTime().domain([start, end]).range([0, 800]);
    const ticks = scale.ticks(12);

    testCases.push({
      name: "time_scale",
      orientation: "bottom",
      domain: [start.toISOString(), end.toISOString()],
      range: [0, 800],
      ticks: ticks.map(t => t.toISOString()),
      tick_positions: ticks.map(t => scale(t))
    });
  }

  // Custom tick values
  {
    const scale = d3.scaleLinear().domain([0, 100]).range([0, 500]);
    const customTicks = [0, 25, 50, 75, 100];

    testCases.push({
      name: "custom_ticks",
      domain: [0, 100],
      range: [0, 500],
      ticks: customTicks,
      tick_positions: customTicks.map(t => scale(t))
    });
  }

  // Tick formatting
  {
    const scale = d3.scaleLinear().domain([0, 1]).range([0, 500]);
    const ticks = [0, 0.25, 0.5, 0.75, 1];
    const format = d3.format(".0%");

    testCases.push({
      name: "tick_format",
      domain: [0, 1],
      range: [0, 500],
      ticks,
      format_specifier: ".0%",
      formatted_ticks: ticks.map(t => format(t))
    });
  }

  // Frequency scale (audio)
  {
    const scale = d3.scaleLog().domain([20, 20000]).range([0, 800]);
    const customTicks = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];

    testCases.push({
      name: "frequency_scale",
      domain: [20, 20000],
      range: [0, 800],
      ticks: customTicks,
      tick_positions: customTicks.map(t => scale(t))
    });
  }

  const golden = createGoldenFile("d3-axis", "axis", testCases);
  fs.mkdirSync(path.join(__dirname, 'axis'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'axis', 'axis.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: axis/axis.json');
}

// ============================================================================
// GEO
// ============================================================================

function generateGeoTests() {
  const testCases = [];

  // Test points for projections
  const testPoints = [
    [0, 0],         // Origin
    [-122.4194, 37.7749],  // San Francisco
    [2.3522, 48.8566],     // Paris
    [139.6917, 35.6895],   // Tokyo
    [-73.9857, 40.7484],   // New York
    [180, 0],       // Date line
    [-180, 0],      // Date line
    [0, 90],        // North pole
    [0, -90],       // South pole
  ];

  // Test multiple projections
  const projections = [
    { name: 'mercator', factory: () => d3.geoMercator() },
    { name: 'equirectangular', factory: () => d3.geoEquirectangular() },
    { name: 'orthographic', factory: () => d3.geoOrthographic() },
    { name: 'stereographic', factory: () => d3.geoStereographic() },
    { name: 'naturalEarth1', factory: () => d3.geoNaturalEarth1() },
    { name: 'equalEarth', factory: () => d3.geoEqualEarth() },
    { name: 'albers', factory: () => d3.geoAlbers() },
  ];

  // Default projection tests
  for (const proj of projections) {
    const projection = proj.factory()
      .scale(100)
      .translate([480, 250]);

    const projected = [];
    const inverted = [];

    for (const point of testPoints) {
      const p = projection(point);
      projected.push(p ? [p[0], p[1]] : null);

      if (p) {
        const inv = projection.invert(p);
        inverted.push(inv ? [inv[0], inv[1]] : null);
      } else {
        inverted.push(null);
      }
    }

    testCases.push({
      name: `${proj.name}_default`,
      projection: proj.name,
      scale: 100,
      translate: [480, 250],
      points: testPoints,
      projected,
      inverted
    });
  }

  // Centered projection test (Mercator centered on Tokyo)
  {
    const projection = d3.geoMercator()
      .center([139.6917, 35.6895])
      .scale(200)
      .translate([400, 300]);

    const projected = testPoints.map(p => {
      const result = projection(p);
      return result ? [result[0], result[1]] : null;
    });

    testCases.push({
      name: 'mercator_centered_tokyo',
      projection: 'mercator',
      center: [139.6917, 35.6895],
      scale: 200,
      translate: [400, 300],
      points: testPoints,
      projected
    });
  }

  // Rotated projection test (Orthographic rotated to show Pacific)
  {
    const projection = d3.geoOrthographic()
      .rotate([120, -30])
      .scale(150)
      .translate([300, 300]);

    const projected = testPoints.map(p => {
      const result = projection(p);
      return result ? [result[0], result[1]] : null;
    });

    testCases.push({
      name: 'orthographic_rotated',
      projection: 'orthographic',
      rotate: [120, -30],
      scale: 150,
      translate: [300, 300],
      points: testPoints,
      projected
    });
  }

  // Clipped extent test
  {
    const projection = d3.geoMercator()
      .scale(100)
      .translate([480, 250])
      .clipExtent([[100, 100], [700, 400]]);

    const projected = testPoints.map(p => {
      const result = projection(p);
      return result ? [result[0], result[1]] : null;
    });

    testCases.push({
      name: 'mercator_clipped',
      projection: 'mercator',
      scale: 100,
      translate: [480, 250],
      clipExtent: [[100, 100], [700, 400]],
      points: testPoints,
      projected
    });
  }

  // geoDistance tests
  {
    const distanceTests = [
      { from: [0, 0], to: [0, 0], name: 'same_point' },
      { from: [0, 0], to: [180, 0], name: 'antipodal' },
      { from: [-122.4194, 37.7749], to: [-73.9857, 40.7484], name: 'sf_to_ny' },
      { from: [2.3522, 48.8566], to: [139.6917, 35.6895], name: 'paris_to_tokyo' },
    ];

    for (const test of distanceTests) {
      testCases.push({
        name: `distance_${test.name}`,
        type: 'distance',
        from: test.from,
        to: test.to,
        distance_radians: d3.geoDistance(test.from, test.to),
        distance_km: d3.geoDistance(test.from, test.to) * 6371  // Earth radius in km
      });
    }
  }

  // geoArea tests
  {
    // Simple polygon (roughly California)
    const california = {
      type: "Polygon",
      coordinates: [[
        [-124.4096, 32.5343],
        [-114.1315, 32.7186],
        [-114.4613, 34.9530],
        [-120.0060, 39.0000],
        [-124.2117, 41.9983],
        [-124.4096, 32.5343]
      ]]
    };

    testCases.push({
      name: 'area_california',
      type: 'area',
      geometry: california,
      area_steradians: d3.geoArea(california),
      area_km2: d3.geoArea(california) * 6371 * 6371  // Approximate
    });
  }

  // geoCentroid tests
  {
    const triangle = {
      type: "Polygon",
      coordinates: [[
        [0, 0],
        [10, 0],
        [5, 10],
        [0, 0]
      ]]
    };

    const centroid = d3.geoCentroid(triangle);
    testCases.push({
      name: 'centroid_triangle',
      type: 'centroid',
      geometry: triangle,
      centroid: [centroid[0], centroid[1]]
    });
  }

  // geoBounds tests
  {
    const polygon = {
      type: "Polygon",
      coordinates: [[
        [-10, -10],
        [20, -10],
        [20, 15],
        [-10, 15],
        [-10, -10]
      ]]
    };

    const bounds = d3.geoBounds(polygon);
    testCases.push({
      name: 'bounds_polygon',
      type: 'bounds',
      geometry: polygon,
      bounds: [[bounds[0][0], bounds[0][1]], [bounds[1][0], bounds[1][1]]]
    });
  }

  // geoPath tests with different projections
  {
    const polygon = {
      type: "Polygon",
      coordinates: [[
        [-10, -10],
        [10, -10],
        [10, 10],
        [-10, 10],
        [-10, -10]
      ]]
    };

    const pathGenerators = [
      { name: 'mercator', projection: d3.geoMercator().scale(100).translate([200, 200]) },
      { name: 'equirectangular', projection: d3.geoEquirectangular().scale(100).translate([200, 200]) },
    ];

    for (const pg of pathGenerators) {
      const pathGen = d3.geoPath(pg.projection);
      testCases.push({
        name: `path_${pg.name}`,
        type: 'path',
        projection: pg.name,
        geometry: polygon,
        path: pathGen(polygon)
      });
    }
  }

  // geoGraticule tests
  {
    const graticule = d3.geoGraticule()
      .step([30, 30])
      .extent([[-180, -90], [180, 90]]);

    const lines = graticule.lines();
    testCases.push({
      name: 'graticule_30deg',
      type: 'graticule',
      step: [30, 30],
      extent: [[-180, -90], [180, 90]],
      line_count: lines.length
    });

    const graticule10 = d3.geoGraticule10();
    testCases.push({
      name: 'graticule10',
      type: 'graticule',
      line_count: d3.geoGraticule().step([10, 10]).lines().length
    });
  }

  // geoRotation tests
  {
    const rotation = d3.geoRotation([90, 0]);
    const testPoint = [0, 0];
    const rotated = rotation(testPoint);
    const inverted = rotation.invert(rotated);

    testCases.push({
      name: 'rotation_90_0',
      type: 'rotation',
      angles: [90, 0],
      input: testPoint,
      rotated: [rotated[0], rotated[1]],
      inverted: [inverted[0], inverted[1]]
    });
  }

  // geoInterpolate tests
  {
    const sf = [-122.4194, 37.7749];
    const ny = [-73.9857, 40.7484];
    const interpolator = d3.geoInterpolate(sf, ny);

    const t_values = [0, 0.25, 0.5, 0.75, 1];
    const interpolated = t_values.map(t => {
      const p = interpolator(t);
      return [p[0], p[1]];
    });

    testCases.push({
      name: 'interpolate_sf_ny',
      type: 'interpolate',
      from: sf,
      to: ny,
      t_values,
      interpolated
    });
  }

  // geoLength tests
  {
    const lineString = {
      type: "LineString",
      coordinates: [
        [-122.4194, 37.7749],  // SF
        [-73.9857, 40.7484]    // NY
      ]
    };

    testCases.push({
      name: 'length_sf_ny',
      type: 'length',
      geometry: lineString,
      length_radians: d3.geoLength(lineString),
      length_km: d3.geoLength(lineString) * 6371
    });
  }

  // geoCircle tests
  {
    const circle = d3.geoCircle()
      .center([-122.4194, 37.7749])  // SF
      .radius(5);  // 5 degrees

    const circleGeoJSON = circle();
    testCases.push({
      name: 'circle_sf_5deg',
      type: 'circle',
      center: [-122.4194, 37.7749],
      radius: 5,
      coordinate_count: circleGeoJSON.coordinates[0].length
    });
  }

  const golden = createGoldenFile("d3-geo", "geo", testCases);
  fs.mkdirSync(path.join(__dirname, 'geo'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'geo', 'geo.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: geo/geo.json');
}

// ============================================================================
// MAIN
// ============================================================================

function generateAllScales() {
  generateLinearScaleTests();
  generateLogScaleTests();
  generatePowScaleTests();
  generateQuantizeScaleTests();
  generateQuantileScaleTests();
  generateThresholdScaleTests();
}

function generateAllInterpolate() {
  generateInterpolateNumberTests();
  generateInterpolateColorTests();
  generateInterpolateStringTests();
}

function generateAllArray() {
  generateArrayStatisticsTests();
  generateArrayBisectTests();
  generateArrayBinTests();
  generateArrayTicksTests();
  generateArrayTransformTests();
}

function generateAllColor() {
  generateColorTests();
}

function generateAllShape() {
  generateArcTests();
  generatePieTests();
  generateLineTests();
  generateSymbolTests();
  generateStackTests();
  generateAreaTests();
}

function generateAllQuadtree() {
  generateQuadtreeTests();
}

function generateAllEase() {
  generateEaseTests();
}

function generateAllContour() {
  generateContourTests();
}

function generateAllDelaunay() {
  generateDelaunayTests();
}

function generateAllBrush() {
  generateBrushTests();
}

function generateAllZoom() {
  generateZoomTests();
}

function generateAllFormat() {
  generateFormatTests();
}

function generateAllTime() {
  generateTimeTests();
}

function generateAllAxis() {
  generateAxisTests();
}

function generateAllGeo() {
  generateGeoTests();
}

function generateAll() {
  console.log(`Generating golden files using D3.js v${d3.version}...\n`);
  generateAllScales();
  generateAllInterpolate();
  generateAllArray();
  generateAllColor();
  generateAllShape();
  generateAllQuadtree();
  generateAllEase();
  generateAllContour();
  generateAllDelaunay();
  generateAllBrush();
  generateAllZoom();
  generateAllFormat();
  generateAllTime();
  generateAllAxis();
  generateAllGeo();
  console.log('\nDone!');
}

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length === 0) {
  generateAll();
} else {
  for (const arg of args) {
    switch (arg) {
      case 'scales':
        generateAllScales();
        break;
      case 'interpolate':
        generateAllInterpolate();
        break;
      case 'array':
        generateAllArray();
        break;
      case 'color':
        generateAllColor();
        break;
      case 'shape':
        generateAllShape();
        break;
      case 'quadtree':
        generateAllQuadtree();
        break;
      case 'ease':
        generateAllEase();
        break;
      case 'contour':
        generateAllContour();
        break;
      case 'delaunay':
        generateAllDelaunay();
        break;
      case 'brush':
        generateAllBrush();
        break;
      case 'zoom':
        generateAllZoom();
        break;
      case 'format':
        generateAllFormat();
        break;
      case 'time':
        generateAllTime();
        break;
      case 'axis':
        generateAllAxis();
        break;
      case 'geo':
        generateAllGeo();
        break;
      default:
        console.error(`Unknown module: ${arg}`);
        process.exit(1);
    }
  }
}
