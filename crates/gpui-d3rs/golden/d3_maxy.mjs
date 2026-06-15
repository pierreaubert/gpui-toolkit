import fs from 'fs';
import { geoConicEqualArea } from './node_modules/d3-geo/src/index.js';
const data = JSON.parse(fs.readFileSync('/tmp/d3_piece_1200.json','utf8'));
const proj = geoConicEqualArea().rotate([60,-60]).parallels([29.5,45.5]).scale(100).translate([0,0]).center([0,0]);
let maxY=-Infinity, maxPt=null, maxGeo=null;
for (const [l,p] of data) {
  const [x,y]=proj([l*180/Math.PI, p*180/Math.PI]);
  if (y>maxY) { maxY=y; maxPt=[l,p]; maxGeo=[l*180/Math.PI,p*180/Math.PI]; }
}
console.log('D3 maxY', maxY, 'rotated', maxPt, 'geo', maxGeo);
