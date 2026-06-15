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

let globalMin = [Infinity, Infinity];
let globalMax = [-Infinity, -Infinity];
let maxPoly = -1;
for (let i = 0; i < multi.coordinates.length; i++) {
  const polygon = { type: 'Polygon', coordinates: multi.coordinates[i] };
  const bounds = geoPath.bounds(polygon);
  if (!Number.isFinite(bounds[0][0])) continue;
  if (bounds[1][1] > globalMax[1]) {
    globalMax = bounds[1];
    globalMin = bounds[0];
    maxPoly = i;
  }
}
console.log('global max polygon', maxPoly, 'bounds', [globalMin, globalMax]);

// print top 10 by max y
const arr = [];
for (let i = 0; i < multi.coordinates.length; i++) {
  const polygon = { type: 'Polygon', coordinates: multi.coordinates[i] };
  const bounds = geoPath.bounds(polygon);
  if (!Number.isFinite(bounds[0][0])) continue;
  arr.push({ i, maxY: bounds[1][1], bounds });
}
arr.sort((a, b) => b.maxY - a.maxY);
for (const e of arr.slice(0, 10)) {
  console.log('poly', e.i, 'maxY', e.maxY, 'bounds', e.bounds);
}
