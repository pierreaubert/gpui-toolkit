//! THROWAWAY oracle comparison (delete after audit).
use d3rs::geo::projection::{
    Albers, ConicEqualArea, Equirectangular, Mercator, Orthographic, Projection, Stereographic,
    TransverseMercator,
};
use d3rs::geo::{GeoJsonGeometry, GeoPath};

const PTS: [[f64; 2]; 9] = [
    [0.0, 0.0],
    [30.0, 20.0],
    [-120.0, 45.0],
    [100.0, -30.0],
    [179.0, 85.0],
    [-45.0, -60.0],
    [10.0, 0.0],
    [-98.0, 38.0],
    [0.0, 60.0],
];

fn oracle() -> Option<serde_json::Value> {
    let s = std::fs::read_to_string("/tmp/d3oracle/oracle.json").ok()?;
    serde_json::from_str(&s).ok()
}

/// Load the node d3-geo oracle, skipping the test when the fixture is
/// absent. Regenerate it with:
/// `cd crates/gpui-d3rs/golden/geo && node oracle_projections.mjs > /tmp/d3oracle/oracle.json`
/// (uses the repo's own d3-geo).
macro_rules! oracle_or_skip {
    () => {
        match oracle() {
            Some(o) => o,
            None => {
                eprintln!("SKIP: /tmp/d3oracle/oracle.json missing; see oracle_or_skip! docs");
                return;
            }
        }
    };
}

fn check(name: &str, expected: &[serde_json::Value], actual: Vec<(f64, f64)>, visible: Vec<bool>) {
    for (i, exp) in expected.iter().enumerate() {
        if exp.is_null() {
            assert!(
                !visible[i],
                "{name}[{i}]: d3 clips but d3rs reports visible ({:?})",
                actual[i]
            );
        } else {
            let ex = exp[0].as_f64().unwrap();
            let ey = exp[1].as_f64().unwrap();
            let (ax, ay) = actual[i];
            assert!(
                (ax - ex).abs() < 1e-6 && (ay - ey).abs() < 1e-6,
                "{name}[{i}]: d3=({ex},{ey}) d3rs=({ax},{ay})"
            );
        }
    }
}

fn project_all<P: Projection>(p: &P) -> (Vec<(f64, f64)>, Vec<bool>) {
    let mut pts = Vec::new();
    let mut vis = Vec::new();
    for [lo, la] in PTS {
        pts.push(p.project(lo, la));
        vis.push(p.is_visible(lo, la));
    }
    (pts, vis)
}

#[test]
fn oracle_plain() {
    let o = oracle_or_skip!();
    let get = |k: &str| o[k].as_array().unwrap().to_vec();

    let p = Mercator::new().scale(150.0).translate(0.0, 0.0);
    let (a, v) = project_all(&p);
    check("mercator.plain", &get("mercator.plain"), a, v);

    let p = Equirectangular::new().scale(150.0).translate(0.0, 0.0);
    let (a, v) = project_all(&p);
    check("equirectangular.plain", &get("equirectangular.plain"), a, v);

    let p = Orthographic::new().scale(150.0).translate(0.0, 0.0);
    let (a, v) = project_all(&p);
    check("orthographic.plain", &get("orthographic.plain"), a, v);

    let p = Stereographic::new().scale(150.0).translate(0.0, 0.0);
    let (a, v) = project_all(&p);
    check("stereographic.plain", &get("stereographic.plain"), a, v);

    let p = TransverseMercator::new().scale(150.0).translate(0.0, 0.0);
    let (a, v) = project_all(&p);
    check(
        "transverseMercator.plain",
        &get("transverseMercator.plain"),
        a,
        v,
    );

    let p = ConicEqualArea::with_parallels(0.0, 60.0)
        .scale(150.0)
        .translate(0.0, 0.0)
        .center(0.0, 33.6442);
    let (a, v) = project_all(&p);
    check("conicEqualArea.plain", &get("conicEqualArea.plain"), a, v);

    let p = Albers::new();
    let pts: Vec<(f64, f64)> = [[-98.0, 38.0], [-120.0, 45.0], [-74.0, 40.0], [-100.0, 30.0]]
        .iter()
        .map(|[lo, la]| p.project(*lo, *la))
        .collect();
    let exp = o["albers.defaults"].as_array().unwrap();
    for (i, e) in exp.iter().enumerate() {
        let (ax, ay) = pts[i];
        let (ex, ey) = (e[0].as_f64().unwrap(), e[1].as_f64().unwrap());
        assert!(
            (ax - ex).abs() < 1e-6 && (ay - ey).abs() < 1e-6,
            "albers[{i}]: d3=({ex},{ey}) d3rs=({ax},{ay})"
        );
    }
}

#[test]
fn oracle_center() {
    let o = oracle_or_skip!();
    let get = |k: &str| o[k].as_array().unwrap().to_vec();

    let p = Mercator::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .center(10.0, 20.0);
    let (a, v) = project_all(&p);
    check("mercator.center", &get("mercator.center"), a, v);

    let p = Equirectangular::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .center(10.0, 20.0);
    let (a, v) = project_all(&p);
    check(
        "equirectangular.center",
        &get("equirectangular.center"),
        a,
        v,
    );

    let p = Orthographic::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .center(10.0, 20.0);
    let (a, v) = project_all(&p);
    check("orthographic.center", &get("orthographic.center"), a, v);

    let p = ConicEqualArea::with_parallels(0.0, 60.0)
        .scale(200.0)
        .translate(100.0, 50.0)
        .center(10.0, 20.0);
    let (a, v) = project_all(&p);
    check("conicEqualArea.center", &get("conicEqualArea.center"), a, v);
}

#[test]
fn oracle_rotate() {
    let o = oracle_or_skip!();
    let get = |k: &str| o[k].as_array().unwrap().to_vec();

    let p = Mercator::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .rotate(30.0, -20.0, 0.0);
    let (a, v) = project_all(&p);
    check("mercator.rotate", &get("mercator.rotate"), a, v);

    let p = Orthographic::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .rotate(30.0, -20.0, 0.0);
    let (a, v) = project_all(&p);
    check("orthographic.rotate", &get("orthographic.rotate"), a, v);

    let p = Mercator::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .rotate(30.0, -20.0, 0.0)
        .center(10.0, 20.0);
    let (a, v) = project_all(&p);
    check("mercator.centerRotate", &get("mercator.centerRotate"), a, v);

    let p = TransverseMercator::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .rotate(30.0, -20.0, 0.0);
    let (a, v) = project_all(&p);
    check("transverse.rotate", &get("transverse.rotate"), a, v);

    let p = TransverseMercator::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .rotate(30.0, -20.0, 45.0);
    let (a, v) = project_all(&p);
    check(
        "transverse.rotateGamma",
        &get("transverse.rotateGamma"),
        a,
        v,
    );
}

#[test]
fn oracle_transverse_center() {
    let o = oracle_or_skip!();
    let get = |k: &str| o[k].as_array().unwrap().to_vec();

    let p = TransverseMercator::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .center(10.0, 20.0);
    let (a, v) = project_all(&p);
    check("transverse.center", &get("transverse.center"), a, v);
}

#[test]
fn oracle_invert() {
    let o = oracle_or_skip!();
    let get = |k: &str| o[k].as_array().unwrap().to_vec();

    let p = Mercator::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .rotate(30.0, -20.0, 0.0)
        .center(10.0, 20.0);
    for (i, e) in get("invert.mercator").iter().enumerate() {
        let xy = if i == 0 {
            (300.0, 100.0)
        } else {
            (100.0, 50.0)
        };
        let (alon, alat) = p.invert(xy.0, xy.1).unwrap();
        let (elon, elat) = (e[0].as_f64().unwrap(), e[1].as_f64().unwrap());
        assert!(
            (alon - elon).abs() < 1e-6 && (alat - elat).abs() < 1e-6,
            "invert.mercator[{i}]: d3=({elon},{elat}) d3rs=({alon},{alat})"
        );
    }

    let p = TransverseMercator::new()
        .scale(200.0)
        .translate(100.0, 50.0)
        .rotate(30.0, -20.0, 0.0)
        .center(10.0, 20.0);
    for (i, e) in get("invert.transverse").iter().enumerate() {
        let xy = if i == 0 {
            (300.0, 100.0)
        } else {
            (100.0, 50.0)
        };
        let (alon, alat) = p.invert(xy.0, xy.1).unwrap();
        let (elon, elat) = (e[0].as_f64().unwrap(), e[1].as_f64().unwrap());
        assert!(
            (alon - elon).abs() < 1e-6 && (alat - elat).abs() < 1e-6,
            "invert.transverse[{i}]: d3=({elon},{elat}) d3rs=({alon},{alat})"
        );
    }
}

fn fit_check(name: &str, o: &serde_json::Value, scale: f64, translate: (f64, f64)) {
    let exp_scale = o["scale"].as_f64().unwrap();
    let exp_t = &o["translate"];
    let (extx, exty) = (exp_t[0].as_f64().unwrap(), exp_t[1].as_f64().unwrap());
    assert!(
        (scale - exp_scale).abs() < 1e-9 * exp_scale.abs().max(1.0),
        "{name}: scale d3={exp_scale} d3rs={scale}"
    );
    assert!(
        (translate.0 - extx).abs() < 1e-9 && (translate.1 - exty).abs() < 1e-9,
        "{name}: translate d3=({extx},{exty}) d3rs={translate:?}"
    );
}

#[test]
fn oracle_fit_all_projections() {
    let o = oracle_or_skip!();
    // CCW ring: d3 reads it as sphere-minus-box, so every projection fits
    // the whole world here.
    let poly = GeoJsonGeometry::Polygon(vec![vec![
        (-10.0, -10.0),
        (10.0, -10.0),
        (10.0, 10.0),
        (-10.0, 10.0),
        (-10.0, -10.0),
    ]]);
    // CW ring: tight box fit.
    let cw = GeoJsonGeometry::Polygon(vec![vec![
        (-10.0, -10.0),
        (-10.0, 10.0),
        (10.0, 10.0),
        (10.0, -10.0),
        (-10.0, -10.0),
    ]]);

    let mut path = GeoPath::new(Mercator::new());
    path.fit_size(300.0, 200.0, &poly);
    fit_check(
        "fitsize.mercator",
        &o["fitsize.mercator"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(Equirectangular::new());
    path.fit_size(300.0, 200.0, &poly);
    fit_check(
        "fitsize.equirectangular",
        &o["fitsize.equirectangular"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(Orthographic::new());
    path.fit_size(300.0, 200.0, &poly);
    fit_check(
        "fitsize.orthographic",
        &o["fitsize.orthographic"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(Stereographic::new());
    path.fit_size(300.0, 200.0, &poly);
    fit_check(
        "fitsize.stereographic",
        &o["fitsize.stereographic"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(ConicEqualArea::with_parallels(0.0, 60.0).center(0.0, 33.6442));
    path.fit_size(300.0, 200.0, &poly);
    fit_check(
        "fitsize.conicEqualArea",
        &o["fitsize.conicEqualArea"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(Albers::new());
    path.fit_size(300.0, 200.0, &poly);
    fit_check(
        "fitsize.albers",
        &o["fitsize.albers"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(TransverseMercator::new());
    path.fit_size(300.0, 200.0, &poly);
    fit_check(
        "fitsize.transverseMercator",
        &o["fitsize.transverseMercator"],
        path.projection().scale(),
        path.projection().translate(),
    );
    let mut path = GeoPath::new(TransverseMercator::new());
    path.fit_extent([[10.0, 20.0], [310.0, 220.0]], &poly);
    fit_check(
        "fitextent.transverseMercator",
        &o["fitextent.transverseMercator"],
        path.projection().scale(),
        path.projection().translate(),
    );

    // Tight fits of the CW box per projection.
    let mut path = GeoPath::new(Mercator::new());
    path.fit_size(300.0, 200.0, &cw);
    fit_check(
        "fitsize_tight.mercator",
        &o["fitsize_tight.mercator"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(Equirectangular::new());
    path.fit_size(300.0, 200.0, &cw);
    fit_check(
        "fitsize_tight.equirectangular",
        &o["fitsize_tight.equirectangular"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(Orthographic::new());
    path.fit_size(300.0, 200.0, &cw);
    fit_check(
        "fitsize_tight.orthographic",
        &o["fitsize_tight.orthographic"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(Stereographic::new());
    path.fit_size(300.0, 200.0, &cw);
    fit_check(
        "fitsize_tight.stereographic",
        &o["fitsize_tight.stereographic"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(ConicEqualArea::with_parallels(0.0, 60.0).center(0.0, 33.6442));
    path.fit_size(300.0, 200.0, &cw);
    fit_check(
        "fitsize_tight.conicEqualArea",
        &o["fitsize_tight.conicEqualArea"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(Albers::new());
    path.fit_size(300.0, 200.0, &cw);
    fit_check(
        "fitsize_tight.albers",
        &o["fitsize_tight.albers"],
        path.projection().scale(),
        path.projection().translate(),
    );

    let mut path = GeoPath::new(TransverseMercator::new());
    path.fit_size(300.0, 200.0, &cw);
    fit_check(
        "fitsize_tight.transverseMercator",
        &o["fitsize_tight.transverseMercator"],
        path.projection().scale(),
        path.projection().translate(),
    );
}
