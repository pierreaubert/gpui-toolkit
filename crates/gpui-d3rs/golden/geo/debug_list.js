const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);

console.log('type', landGeojson.type, 'features', landGeojson.features.length);
for (let i = 0; i < Math.min(10, landGeojson.features.length); i++) {
  const f = landGeojson.features[i];
  let minlon = Infinity, maxlon = -Infinity, minlat = Infinity, maxlat = -Infinity;
  const coords = f.geometry.coordinates;
  const rings = f.geometry.type === 'Polygon' ? coords : coords[0];
  for (const ring of rings) {
    for (const [lon, lat] of ring) {
      if (lon < minlon) minlon = lon;
      if (lon > maxlon) maxlon = lon;
      if (lat < minlat) minlat = lat;
      if (lat > maxlat) maxlat = lat;
    }
  }
  console.log(i, f.geometry.type, 'bbox', minlon, maxlon, minlat, maxlat);
}
