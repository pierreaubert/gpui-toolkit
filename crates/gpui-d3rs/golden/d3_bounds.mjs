import fs from 'fs';
import topojson from 'topojson-client';
import { geoConicEqualArea, geoPath } from './node_modules/d3-geo/src/index.js';
const data = JSON.parse(fs.readFileSync('/Volumes/home_ext1/src_pierre/gpui-toolkit/crates/gpui-d3rs/bin/showcase/data/land-50m.json','utf8'));
const land = topojson.feature(data, data.objects.land).features[0];
const cases = [
  [0,-15], [60,-60], [0,0], [15,-15], [30,15], [45,-75]
];
for (const [lon,lat] of cases) {
  const proj = geoConicEqualArea().rotate([lon, lat]).parallels([29.5,45.5]).scale(100).translate([0,0]).center([0,0]);
  const path = geoPath(proj);
  const b = path.bounds(land);
  console.log(lon, lat, JSON.stringify(b));
}
