/**
 * Golden file generator for geographic projections at varied angles.
 *
 * Generates expected forward-projection values from D3.js for the five
 * projections used in gpui-d3rs, exercising both center (longitude/latitude)
 * and rotation (lambda/phi/gamma) angles.
 */

const d3 = require('d3');
const fs = require('fs');
const path = require('path');

const TOLERANCE = 1e-6;

function round(x) {
  return Math.round(x * 1e9) / 1e9;
}

function createGoldenFile(testCases) {
  return {
    module: 'd3-geo',
    function: 'projections_angles',
    d3_version: d3.version,
    tolerance: TOLERANCE,
    generated_at: new Date().toISOString(),
    test_cases: testCases,
  };
}

const PROJECTIONS = {
  mercator: () => d3.geoMercator(),
  equirectangular: () => d3.geoEquirectangular(),
  orthographic: () => d3.geoOrthographic(),
  stereographic: () => d3.geoStereographic(),
  conicEqualArea: () => d3.geoConicEqualArea().parallels([29.5, 45.5]),
};

const SCALE = 100;
const TRANSLATE = [0, 0];

const ANGLE_CASES = [
  { center: [0, 0], rotate: [0, 0, 0], name_suffix: 'default' },
  { center: [30, 0], rotate: [0, 0, 0], name_suffix: 'center_lon_30' },
  { center: [-30, 0], rotate: [0, 0, 0], name_suffix: 'center_lon_-30' },
  { center: [0, 15], rotate: [0, 0, 0], name_suffix: 'center_lat_15' },
  { center: [0, -15], rotate: [0, 0, 0], name_suffix: 'center_lat_-15' },
  { center: [30, -15], rotate: [0, 0, 0], name_suffix: 'center_30_-15' },
  { center: [0, 0], rotate: [30, -15, 0], name_suffix: 'rotate_30_-15' },
  { center: [0, 0], rotate: [-30, 15, 0], name_suffix: 'rotate_-30_15' },
  { center: [0, 0], rotate: [0, 0, 15], name_suffix: 'rotate_gamma_15' },
  { center: [30, -15], rotate: [-30, 15, 15], name_suffix: 'center_and_rotate' },
];

const POINTS = [
  [0, 0],
  [30, 30],
  [-30, -30],
  [120, 45],
  [-80, -20],
];

function generate() {
  const testCases = [];

  for (const [projectionName, factory] of Object.entries(PROJECTIONS)) {
    for (const angleCase of ANGLE_CASES) {
      const projection = factory()
        .scale(SCALE)
        .translate(TRANSLATE)
        .center(angleCase.center)
        .rotate(angleCase.rotate);

      const projected = POINTS.map((pt) => {
        const [x, y] = projection(pt);
        return [round(x), round(y)];
      });

      testCases.push({
        name: `${projectionName}_${angleCase.name_suffix}`,
        projection: projectionName,
        scale: SCALE,
        translate: TRANSLATE,
        center: angleCase.center,
        rotate: angleCase.rotate,
        points: POINTS,
        projected,
      });
    }
  }

  const golden = createGoldenFile(testCases);
  const outPath = path.join(__dirname, 'projections_angles.json');
  fs.writeFileSync(outPath, JSON.stringify(golden, null, 2));
  console.log(`Generated: ${outPath} (${testCases.length} cases)`);
}

generate();
