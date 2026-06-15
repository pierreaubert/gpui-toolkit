const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);

function createProjection(name) {
  switch (name) {
    case 'orthographic': return d3.geoOrthographic();
    case 'stereographic': return d3.geoStereographic();
    default: throw new Error(name);
  }
}

function debugProjection(name, rotate, clipAngle) {
  const projection = createProjection(name)
    .scale(100)
    .translate([0, 0])
    .center([0, 0])
    .rotate(rotate)
    .clipAngle(clipAngle);

  const radius = clipAngle * Math.PI / 180;
  const startRotated = [0, -radius * 180 / Math.PI]; // degrees
  const startGeo = projection.invert(projection(startRotated));
  console.log(name, 'rotate', rotate, 'clipAngle', clipAngle);
  console.log('startRotated', startRotated, 'startGeo', startGeo);

  let count = 0;
  const features = landGeojson.features || [landGeojson];
  for (let i = 0; i < features.length; i++) {
    const f = features[i];
    if (d3.geoContains(f, startGeo)) {
      count++;
      console.log('  contains polygon', i, f.properties);
    }
  }
  console.log('total containing polygons', count);
  const geoPath = d3.geoPath(projection);
  const bounds = geoPath.bounds(landGeojson);
  console.log('bounds', bounds);
}

debugProjection('orthographic', [45, -15, 0], 90 + 1e-6);
debugProjection('stereographic', [45, -15, 0], 90 + 1e-6);
