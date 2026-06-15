/**
 * Golden file generator for geographic projection path rendering.
 *
 * Renders the full land-50m world map with d3-geo at a grid of angles and
 * records compact, comparable path statistics. Cylindrical projections use
 * .center(lon, lat) for panning; azimuthal/conic projections use
 * .rotate(lon, lat, 0).
 */

const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const TOLERANCE = 1e-3;
const SCALE = 100;
const TRANSLATE = [0, 0];
const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const OUT_PATH = path.join(__dirname, 'land_projection_paths.json');

const CYLINDRICAL = ['mercator', 'equirectangular'];
const AZIMUTHAL_CONIC = ['orthographic', 'stereographic', 'conicEqualArea'];

function createProjection(name) {
  switch (name) {
    case 'mercator':
      return d3.geoMercator();
    case 'equirectangular':
      return d3.geoEquirectangular();
    case 'orthographic':
      return d3.geoOrthographic();
    case 'stereographic':
      return d3.geoStereographic();
    case 'conicEqualArea':
      return d3.geoConicEqualArea().parallels([29.5, 45.5]);
    default:
      throw new Error(`unknown projection: ${name}`);
  }
}

function round2(x) {
  return Math.round(x * 1e6) / 1e6;
}

function generate() {
  const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
  const landGeojson = topojson.feature(topology, topology.objects.land);

  const testCases = [];

  for (let lon = 0; lon < 360; lon += 15) {
    for (let lat = -90; lat <= 90; lat += 15) {
      for (const projectionName of CYLINDRICAL) {
        if (projectionName === 'mercator' && Math.abs(lat) === 90) {
          continue;
        }

        const projection = createProjection(projectionName)
          .scale(SCALE)
          .translate(TRANSLATE)
          .center([lon, lat])
          .rotate([0, 0, 0]);

        const geoPath = d3.geoPath(projection);
        const bounds = geoPath.bounds(landGeojson);
        const centroid = geoPath.centroid(landGeojson);

        if (!Number.isFinite(bounds[0][0]) || !Number.isFinite(bounds[1][0])) {
          continue;
        }

        testCases.push({
          name: `${projectionName}_lon${lon}_lat${lat}`,
          projection: projectionName,
          lon,
          lat,
          scale: SCALE,
          translate: TRANSLATE,
          center: [lon, lat],
          rotate: [0, 0, 0],
          bounds: [
            [round2(bounds[0][0]), round2(bounds[0][1])],
            [round2(bounds[1][0]), round2(bounds[1][1])],
          ],
          centroid: [round2(centroid[0]), round2(centroid[1])],
        });
      }

      for (const projectionName of AZIMUTHAL_CONIC) {
        const projection = createProjection(projectionName)
          .scale(SCALE)
          .translate(TRANSLATE)
          .center([0, 0])
          .rotate([lon, lat, 0]);

        const geoPath = d3.geoPath(projection);
        const bounds = geoPath.bounds(landGeojson);
        const centroid = geoPath.centroid(landGeojson);

        testCases.push({
          name: `${projectionName}_lon${lon}_lat${lat}`,
          projection: projectionName,
          lon,
          lat,
          scale: SCALE,
          translate: TRANSLATE,
          center: [0, 0],
          rotate: [lon, lat, 0],
          bounds: [
            [round2(bounds[0][0]), round2(bounds[0][1])],
            [round2(bounds[1][0]), round2(bounds[1][1])],
          ],
          centroid: [round2(centroid[0]), round2(centroid[1])],
        });
      }
    }
  }

  const golden = {
    module: 'd3-geo',
    function: 'land_projection_paths',
    d3_version: d3.version,
    tolerance: TOLERANCE,
    generated_at: new Date().toISOString(),
    test_cases: testCases,
  };

  fs.writeFileSync(OUT_PATH, JSON.stringify(golden, null, 2));
  console.log(`Generated: ${OUT_PATH} (${testCases.length} cases)`);
}

generate();
