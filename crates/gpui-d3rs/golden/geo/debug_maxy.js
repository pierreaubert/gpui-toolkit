const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);
const multi = landGeojson.features[0].geometry;

const projection = d3.geoConicEqualArea()
  .parallels([29.5, 45.5])
  .scale(100)
  .translate([0, 0])
  .center([0, 0])
  .rotate([60, -60, 0]);

const geoPath = d3.geoPath(projection);
const polygon = { type: 'Polygon', coordinates: multi.coordinates[1200] };
const p = geoPath(polygon);
// Parse path for coordinates
const coords = [];
let current = null;
const tokens = p.match(/[MLZ][^MLZ]*/g);
for (const tok of tokens) {
  const cmd = tok[0];
  const rest = tok.slice(1).trim();
  if (!rest) continue;
  const nums = rest.split(/[ ,]/).map(Number);
  for (let i = 0; i < nums.length; i += 2) {
    current = [nums[i], nums[i + 1]];
    coords.push(current);
  }
}
let maxY = -Infinity, maxPt = null;
for (const c of coords) {
  if (c[1] > maxY) {
    maxY = c[1];
    maxPt = c;
  }
}
console.log('max projected', maxPt, 'y', maxY);
console.log('inverse geo', projection.invert(maxPt));
console.log('inverse rotated', d3.geoRotation([60,-60,0]).invert(projection.invert(maxPt)));
