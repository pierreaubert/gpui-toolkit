use gpui_profiler::{AllocProbe, AllocSnapshot, samples_to_chrome_trace, samples_to_csv};

fn main() {
    let mut probe = AllocProbe::new();
    let mut series = Vec::new();

    // Leave probes in hot-path code, then opt into real allocation counting only
    // for profiling builds with `--features global-allocator`.
    probe.reset();
    do_steady_work();
    series.push(probe.sample_labeled("steady-work"));
    report("steady-work", probe.sample("steady-work"));

    probe.reset();
    let values = do_allocating_work();
    series.push(probe.sample_labeled("allocating-work"));
    report("allocating-work", probe.sample("allocating-work"));
    std::hint::black_box(values);

    println!("peak sample bytes: {}", probe.peak_bytes());
    println!("--- csv ---\n{}", samples_to_csv(&series));
    println!("--- chrome trace ---\n{}", samples_to_chrome_trace(&series));

    #[cfg(feature = "global-allocator")]
    eprintln!("allocation counting is enabled");

    #[cfg(not(feature = "global-allocator"))]
    eprintln!("allocation counting is disabled; samples are zero-cost placeholders");
}

fn do_steady_work() {
    let mut total = 0usize;
    for value in 0..128 {
        total = total.wrapping_add(value);
    }
    std::hint::black_box(total);
}

fn do_allocating_work() -> Vec<usize> {
    let mut values = Vec::new();
    values.extend(0..128);
    values
}

fn report(label: &str, snapshot: AllocSnapshot) {
    println!(
        "{label}: {} allocations ({} fresh, {} reallocs), {} bytes",
        snapshot.count,
        snapshot.allocs(),
        snapshot.reallocs,
        snapshot.bytes
    );
}
