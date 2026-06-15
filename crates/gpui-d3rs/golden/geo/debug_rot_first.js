const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);
const multi = landGeojson.features[0].geometry;
const ring = multi.coordinates[1200][0];
const rot = d3.geoRotation([60, -60, 0]);
console.log('first geo', ring[0], 'rot', rot(ring[0]).map(x=>x));
console.log('last geo', ring[ring.length-1], 'rot', rot(ring[ring.length-1]).map(x=>x));
// Find a point near south after rotation
let minPhi = Infinity, minPt = null;
for (const p of ring) {
  const r = rot(p);
  if (r[1] < minPhi) { minPhi = r[1]; minPt = p; }
}
console.log('min phi geo', minPt, 'rot', rot(minPt).map(x=>x));
