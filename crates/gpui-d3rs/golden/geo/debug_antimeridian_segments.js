const clip = require('../node_modules/d3-geo/src/clip/index.js').default;
const clipAntimeridian = require('../node_modules/d3-geo/src/clip/antimeridian.js').default;
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);
const multi = landGeojson.features[0].geometry;

const polygon = multi.coordinates[1200];
const rotate = [60, -60, 0];
const rot = require('d3').geoRotation(rotate);

function runClip() {
  const segments = [];
  let currentLine = null;
  const sink = {
    polygonStart() {},
    polygonEnd() {},
    lineStart() { currentLine = []; },
    lineEnd() { if (currentLine) segments.push(currentLine); currentLine = null; },
    point(x, y) { currentLine.push([x, y]); },
  };

  const clipStream = clipAntimeridian(sink);
  clipStream.polygonStart();
  for (const ring of polygon) {
    clipStream.lineStart();
    for (const p of ring) {
      const r = rot(p);
      clipStream.point(r[0] * Math.PI / 180, r[1] * Math.PI / 180);
    }
    clipStream.lineEnd();
  }
  clipStream.polygonEnd();

  console.log('segments', segments.length);
  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    console.log('seg', i, 'len', seg.length, 'first', seg[0].map(x => x * 180 / Math.PI), 'last', seg[seg.length - 1].map(x => x * 180 / Math.PI));
  }
}

runClip();
