import fs from 'fs';
import topojson from 'topojson-client';
import { geoConicEqualArea, geoPath } from './node_modules/d3-geo/src/index.js';
const data = JSON.parse(fs.readFileSync('/Volumes/home_ext1/src_pierre/gpui-toolkit/crates/gpui-d3rs/bin/showcase/data/land-50m.json','utf8'));
const land = topojson.feature(data, data.objects.land).features[0];
const proj = geoConicEqualArea().rotate([60,-60]).parallels([29.5,45.5]).scale(100).translate([0,0]).center([0,0]);
const path = geoPath(proj);
let globalMax = -Infinity, globalIdx=-1, globalPt=null;
for (let i=0;i<land.geometry.coordinates.length;i++) {
  const poly = {type:'Feature', geometry:{type:'Polygon', coordinates:land.geometry.coordinates[i]}, properties:{}};
  const b = path.bounds(poly);
  if (b[1][1] > globalMax) {
    globalMax = b[1][1];
    globalIdx = i;
    globalPt = b[1];
  }
}
console.log('polygon', globalIdx, 'max y', globalMax, 'bounds top-right', globalPt);
// print this polygon's projected path bounds and a few points
const poly = {type:'Feature', geometry:{type:'Polygon', coordinates:land.geometry.coordinates[globalIdx]}, properties:{}};
console.log('bounds', path.bounds(poly));
