use criterion::{Criterion, black_box, criterion_group, criterion_main};
use d3rs::geo::{Equirectangular, GeoJsonGeometry, GeoPath};
use d3rs::shape::path::PathBuilder;

fn bench_path_to_svg_string(c: &mut Criterion) {
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .line_to(100.0, 0.0)
        .line_to(100.0, 100.0)
        .cubic_curve_to(120.0, 120.0, 80.0, 120.0, 100.0, 150.0)
        .close_path()
        .build();

    c.bench_function("path/to_svg_string", |b| {
        b.iter(|| black_box(path.to_svg_string()));
    });

    let mut scratch = String::with_capacity(256);
    c.bench_function("path/write_svg_string", |b| {
        b.iter(|| {
            scratch.clear();
            path.write_svg_string(&mut scratch);
            black_box(&scratch);
        });
    });
}

fn bench_geo_path_render(c: &mut Criterion) {
    let projection = Equirectangular::new().scale(100.0).translate(0.0, 0.0);
    let path = GeoPath::new(projection);
    let geometry = GeoJsonGeometry::LineString(vec![
        (0.0, 0.0),
        (10.0, 10.0),
        (20.0, 0.0),
        (30.0, 10.0),
        (40.0, 0.0),
    ]);

    c.bench_function("geo_path/render", |b| {
        b.iter(|| black_box(path.render(black_box(&geometry))));
    });

    let mut scratch = String::with_capacity(256);
    c.bench_function("geo_path/render_into", |b| {
        b.iter(|| {
            scratch.clear();
            path.render_into(black_box(&geometry), &mut scratch);
            black_box(&scratch);
        });
    });
}

criterion_group!(benches, bench_path_to_svg_string, bench_geo_path_render);
criterion_main!(benches);
