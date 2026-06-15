const d3 = require('d3');
const fs = require('fs');
const path = require('path');

function makePath(projType, scale, translate, geometry) {
  const factory = {
    mercator: () => d3.geoMercator(),
    equirectangular: () => d3.geoEquirectangular(),
  }[projType];
  const projection = factory().scale(scale).translate(translate);
  const path = d3.geoPath(projection);
  return path({ type: geometry.type, coordinates: geometry.coordinates });
}

const cases = [
  {
    name: 'mercator_square',
    projection: 'mercator',
    scale: 100,
    translate: [200, 200],
    geometry: { type: 'Polygon', coordinates: [[[-10, -10], [-10, 10], [10, 10], [10, -10], [-10, -10]]] },
  },
  {
    name: 'mercator_antarctica_clip',
    projection: 'mercator',
    scale: 100,
    translate: [300, 200],
    geometry: { type: 'Polygon', coordinates: [[[-180, -90], [-180, -60], [-90, -65], [0, -70], [90, -65], [180, -60], [180, -90], [-180, -90]]] },
  },
  {
    name: 'equirectangular_square',
    projection: 'equirectangular',
    scale: 100,
    translate: [200, 200],
    geometry: { type: 'Polygon', coordinates: [[[-10, -10], [-10, 10], [10, 10], [10, -10], [-10, -10]]] },
  },
  {
    name: 'equirectangular_antimeridian_polygon',
    projection: 'equirectangular',
    scale: 1,
    translate: [0, 0],
    geometry: { type: 'Polygon', coordinates: [[[170, 65], [175, 60], [-175, 60], [-170, 65], [170, 65]]] },
  },
];

const golden = {
  module: 'd3-geo',
  function: 'path_cylindrical',
  d3_version: d3.version,
  tolerance: 0.001,
  generated_at: new Date().toISOString(),
  test_cases: cases.map(c => ({
    name: c.name,
    projection: c.projection,
    scale: c.scale,
    translate: c.translate,
    geometry: c.geometry,
    path: makePath(c.projection, c.scale, c.translate, c.geometry),
  })),
};

const outPath = path.join(__dirname, 'path_cylindrical.json');
fs.writeFileSync(outPath, JSON.stringify(golden, null, 2));
console.log(`Generated: ${outPath}`);
