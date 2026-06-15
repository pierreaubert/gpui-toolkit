import { geoConicEqualArea, geoPath } from './node_modules/d3-geo/src/index.js';
const proj = geoConicEqualArea().rotate([0,-15]).parallels([29.5,45.5]).scale(100).translate([0,0]).center([0,0]);
let maxX=-Infinity, pt=null;
for (let phi=-90; phi<=90; phi+=0.001) {
  const [x,y]=proj([180,phi]);
  if (x>maxX){maxX=x; pt=[180,phi];}
}
console.log('maxX', maxX, 'at', pt, 'proj', proj(pt));
