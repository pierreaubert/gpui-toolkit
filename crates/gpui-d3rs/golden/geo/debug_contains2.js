const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);

const rotate = d3.geoRotation([45, -15, 0]);
const clipAngle = 90 + 1e-6;
const radius = clipAngle * Math.PI / 180;
// D3's clip start for smallRadius false (cr <= 0) is [-pi, radius - pi]
const startRotated = [-180, (radius - Math.PI) * 180 / Math.PI];
console.log('startRotated', startRotated);

function rotatePolygon(poly) {
  if (!Array.isArray(poly[0][0])) return poly.map(ring => ring.map(p => rotate(p)));
  return poly.map(polygon => rotatePolygon(polygon));
}

let count = 0;
for (let i = 0; i < landGeojson.features.length; i++) {
  const f = landGeojson.features[i];
  const rotated = rotatePolygon(f.geometry.coordinates);
  const geom = f.geometry.type === 'Polygon'
    ? { type: 'Polygon', coordinates: rotated }
    : { type: 'MultiPolygon', coordinates: rotated };
  if (d3.geoContains(geom, startRotated)) {
    count++;
    console.log('contains feature', i, f.geometry.type);
    const rings = f.geometry.type === 'Polygon' ? f.geometry.coordinates : f.geometry.coordinates[0];
    let minlon = Infinity, maxlon = -Infinity, minlat = Infinity, maxlat = -Infinity;
    for (const ring of rings) {
      for (const [lon, lat] of ring) {
        if (lon < minlon) minlon = lon;
        if (lon > maxlon) maxlon = lon;
        if (lat < minlat) minlat = lat;
        if (lat > maxlat) maxlat = lat;
      }
    }
    console.log('  geo bbox lon', minlon, maxlon, 'lat', minlat, maxlat, 'rings', rings.length);
  }
}
console.log('total', count);
