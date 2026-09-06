use d3rs::geo::projection::{Projection, TransverseMercator};
#[test]
fn debug_transverse() {
    for rot in [[0.0, 0.0, 0.0], [0.0, 0.0, 90.0]] {
        let p = TransverseMercator::new()
            .scale(150.0)
            .translate(0.0, 0.0)
            .rotate(rot[0], rot[1], rot[2]);
        for [lo, la] in [[30.0, 20.0], [-120.0, 45.0]] {
            let (x, y) = p.project(lo, la);
            println!("rot={rot:?} [{lo},{la}] -> [{x:.6},{y:.6}]");
        }
    }
}
