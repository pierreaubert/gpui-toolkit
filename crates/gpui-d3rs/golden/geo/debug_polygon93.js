const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);

const projection = d3.geoOrthographic()
  .scale(100)
  .translate([0, 0])
  .center([0, 0])
  .rotate([45, -15, 0]);

const geoPath = d3.geoPath(projection);

const multi = landGeojson.features[0].geometry;
console.log('multipolygon polygons', multi.coordinates.length);

for (const idx of [93, 1200]) {
  const polygon = { type: 'Polygon', coordinates: multi.coordinates[idx] };
  let minlon = Infinity, maxlon = -Infinity, minlat = Infinity, maxlat = -Infinity;
  for (const ring of multi.coordinates[idx]) {
    for (const [lon, lat] of ring) {
      if (lon < minlon) minlon = lon;
      if (lon > maxlon) maxlon = lon;
      if (lat < minlat) minlat = lat;
      if (lat > maxlat) maxlat = lat;
    }
  }
  console.log('\npolygon', idx, 'rings', multi.coordinates[idx].length, 'bbox', minlon, maxlon, minlat, maxlat);
  const bounds = geoPath.bounds(polygon);
  console.log('projected bounds', bounds);
  const p = geoPath(polygon);
  console.log('path len', p.length, 'prefix', p.slice(0, 300));
}
