use criterion::{Criterion, criterion_group, criterion_main};
use gpui_ios::momentum::{MomentumScroller, VelocityTracker};

fn bench_momentum(c: &mut Criterion) {
    let mut group = c.benchmark_group("momentum");
    group.bench_function("velocity_tracker_record", |b| {
        let mut tracker = VelocityTracker::new();
        b.iter(|| tracker.record(1.0, 2.0));
    });
    // Fresh samples per iteration: `velocity()` only considers samples newer
    // than its 100ms window, so reusing one batch would decay into the
    // early-return path and stop measuring the regression fit.
    group.bench_function("velocity_tracker_record_20_then_velocity", |b| {
        b.iter(|| {
            let mut tracker = VelocityTracker::new();
            for index in 0..20 {
                tracker.record(index as f32 * 8.0, 0.0);
            }
            tracker.velocity()
        });
    });
    group.bench_function("momentum_scroller_fling_step", |b| {
        let mut scroller = MomentumScroller::new();
        b.iter(|| {
            scroller.fling(2_000.0, 0.0, 0.0, 0.0);
            scroller.step()
        });
    });
    group.finish();
}

criterion_group!(benches, bench_momentum);
criterion_main!(benches);
