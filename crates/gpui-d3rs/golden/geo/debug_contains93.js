const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);
const multi = landGeojson.features[0].geometry;

const rotate = d3.geoRotation([45, -15, 0]);
const clipAngle = 90 + 1e-6;
const radius = clipAngle * Math.PI / 180;
const startRotated = [-180, (radius - Math.PI) * 180 / Math.PI];
console.log('startRotated', startRotated);

function rotatePolygon(coords) {
  if (!Array.isArray(coords[0][0])) return coords.map(ring => ring.map(p => rotate(p)));
  return coords.map(polygon => rotatePolygon(polygon));
}

for (const idx of [93, 1200]) {
  const coords = multi.coordinates[idx];
  const rotated = rotatePolygon(coords);
  const geom = { type: 'Polygon', coordinates: rotated };
  const contains = d3.geoContains(geom, startRotated);
  console.log('polygon', idx, 'contains start:', contains, 'rings', coords.length);
}
