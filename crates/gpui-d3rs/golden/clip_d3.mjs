import { rotateRadians } from './node_modules/d3-geo/src/rotation.js';
import clipAntimeridian from './node_modules/d3-geo/src/clip/antimeridian.js';
import { geoTransform } from './node_modules/d3-geo/src/index.js';
import fs from 'fs';
import topojson from 'topojson-client';

const data = JSON.parse(fs.readFileSync('/Volumes/home_ext1/src_pierre/gpui-toolkit/crates/gpui-d3rs/bin/showcase/data/land-50m.json', 'utf8'));
const land = topojson.feature(data, data.objects.land).features[0];
const polygon = land.geometry.coordinates[1200];
console.log('rings:', polygon.length, 'outer len', polygon[0].length);

const rotate = rotateRadians(60 * Math.PI/180, -60 * Math.PI/180, 0);
const rotationStream = geoTransform({
  point(lambda, phi) { this.stream.point(...rotate(lambda, phi)); }
});

function clipAndCollect() {
  let pieces = [];
  let currentPiece = null;
  let currentRing = null;
  const outputSink = {
    point(x, y, z) { if (currentRing) currentRing.push([x, y]); },
    lineStart() { currentRing = []; },
    lineEnd() { if (currentPiece && currentRing) currentPiece.push(currentRing); currentRing = null; },
    polygonStart() { currentPiece = []; },
    polygonEnd() { if (currentPiece !== null) pieces.push(currentPiece); currentPiece = null; }
  };
  const clip = clipAntimeridian(outputSink);
  const stream = rotationStream.stream(clip);
  stream.polygonStart();
  for (const ring of polygon) {
    stream.lineStart();
    for (const [lon, lat] of ring) {
      stream.point(lon * Math.PI/180, lat * Math.PI/180);
    }
    stream.lineEnd();
  }
  stream.polygonEnd();
  return pieces;
}

const pieces = clipAndCollect();
console.log('D3 pieces:', pieces.length);
for (let i = 0; i < pieces.length; i++) {
  const piece = pieces[i];
  console.log('piece', i, 'rings', piece.length);
  for (let j = 0; j < piece.length; j++) {
    const ring = piece[j];
    console.log('  ring', j, 'len', ring.length, 'first', JSON.stringify(ring[0]), 'last', JSON.stringify(ring[ring.length-1]));
  }
}

// Write outer piece coords to file (radians)
const outer = pieces[0][0];
fs.writeFileSync('/tmp/d3_piece_1200.json', JSON.stringify(outer.map(([l,p])=>[l,p])));
console.log('wrote /tmp/d3_piece_1200.json');
