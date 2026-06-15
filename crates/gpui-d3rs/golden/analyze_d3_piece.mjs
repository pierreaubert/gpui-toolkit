import { rotateRadians } from './node_modules/d3-geo/src/rotation.js';
import fs from 'fs';
const data = JSON.parse(fs.readFileSync('/tmp/d3_piece_1200.json','utf8'));
const rotate = rotateRadians(60 * Math.PI/180, -60 * Math.PI/180, 0);
let maxPhi = -Infinity, minPhi = Infinity, maxLon=-Infinity, minLon=Infinity;
let maxPt=null, minPt=null;
for (const [l,p] of data) {
  const [lon, lat] = rotate.invert(l, p).map(v=>v*180/Math.PI);
  if (lat > maxPhi) { maxPhi=lat; maxPt=[lon,lat]; }
  if (lat < minPhi) { minPhi=lat; minPt=[lon,lat]; }
  if (lon > maxLon) maxLon=lon;
  if (lon < minLon) minLon=lon;
}
console.log('D3 piece unrotated lat range', minPhi, maxPhi, 'lon range', minLon, maxLon);
console.log('max lat pt', maxPt, 'min lat pt', minPt);
console.log('first unrotated', rotate.invert(data[0][0], data[0][1]).map(v=>v*180/Math.PI));
console.log('last unrotated', rotate.invert(data[data.length-1][0], data[data.length-1][1]).map(v=>v*180/Math.PI));
