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
console.log('bounds', geoPath.bounds(polygon));
console.log('path len', p.length);
console.log(p.slice(0, 500));
