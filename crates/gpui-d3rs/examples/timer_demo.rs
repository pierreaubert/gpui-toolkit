//! Timer module demo
//!
//! Run with: cargo run --example timer_demo
//!
//! This demonstrates the d3-timer module functionality:
//! - now() - monotonic time since epoch
//! - timer() - repeating callback at ~60fps
//! - timeout() - one-shot delayed callback
//! - interval() - repeating callback at fixed intervals

use d3rs::timer::{Interval, Timeout, Timer, interval, now, timeout, timer, timer_flush};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== d3-timer Demo ===\n");

    // Initialize the epoch and demonstrate now()
    println!("--- now() Demo ---");
    let t0 = now();
    println!("Initial time: {:.2} ms", t0);
    thread::sleep(Duration::from_millis(100));
    let t1 = now();
    println!("After 100ms sleep: {:.2} ms", t1);
    println!("Elapsed: {:.2} ms (expected ~100)\n", t1 - t0);

    // timer_flush() demo
    println!("--- timer_flush() Demo ---");
    timer_flush();
    println!("Timer system initialized\n");

    // Timer demo - runs at ~60fps
    println!("--- Timer Demo (60fps) ---");
    println!("Creating a timer that runs for ~200ms...");

    let start = now();
    let tick_count = Arc::new(AtomicUsize::new(0));
    let tick_count_clone = tick_count.clone();

    let t = timer(
        move |elapsed| {
            tick_count_clone.fetch_add(1, Ordering::SeqCst);
            elapsed < 200.0 // Run for ~200ms
        },
        None,
        None,
    );

    // Wait for timer to complete
    t.join();

    let total_elapsed = now() - start;
    let ticks = tick_count.load(Ordering::SeqCst);
    println!(
        "Timer completed: {} ticks in {:.1}ms ({:.1} ticks/sec)\n",
        ticks,
        total_elapsed,
        ticks as f64 / (total_elapsed / 1000.0)
    );

    // Timer with delay demo
    println!("--- Timer with Delay Demo ---");
    println!("Creating a timer with 50ms initial delay...");

    let start = now();
    let first_tick_time = Arc::new(std::sync::Mutex::new(0.0_f64));
    let first_tick_clone = first_tick_time.clone();

    let t = Timer::new(
        move |elapsed| {
            let mut first = first_tick_clone.lock().unwrap();
            if *first == 0.0 {
                *first = elapsed;
            }
            false // Fire once
        },
        Some(50.0), // 50ms delay
        None,
    );

    t.join();

    let total = now() - start;
    let first = *first_tick_time.lock().unwrap();
    println!(
        "First tick at {:.1}ms elapsed (total runtime: {:.1}ms)\n",
        first, total
    );

    // Timeout demo
    println!("--- Timeout Demo ---");
    println!("Creating a timeout that fires after 100ms...");

    let start = now();
    let fired = Arc::new(std::sync::Mutex::new(false));
    let fired_clone = fired.clone();
    let fire_time = Arc::new(std::sync::Mutex::new(0.0_f64));
    let fire_time_clone = fire_time.clone();

    let t = timeout(
        move |elapsed| {
            *fired_clone.lock().unwrap() = true;
            *fire_time_clone.lock().unwrap() = elapsed;
        },
        100.0,
        None,
    );

    t.join();

    let total = now() - start;
    let did_fire = *fired.lock().unwrap();
    let fire_at = *fire_time.lock().unwrap();
    println!(
        "Timeout fired: {} at {:.1}ms (total: {:.1}ms)\n",
        did_fire, fire_at, total
    );

    // Timeout cancellation demo
    println!("--- Timeout Cancellation Demo ---");
    println!("Creating a 500ms timeout and cancelling after 50ms...");

    let fired = Arc::new(AtomicUsize::new(0));
    let fired_clone = fired.clone();

    let t = Timeout::new(
        move |_| {
            fired_clone.fetch_add(1, Ordering::SeqCst);
        },
        500.0,
        None,
    );

    thread::sleep(Duration::from_millis(50));
    t.stop();
    println!("Timeout stopped");

    thread::sleep(Duration::from_millis(100));
    let fire_count = fired.load(Ordering::SeqCst);
    println!("Fire count after stop: {} (should be 0)\n", fire_count);

    // Interval demo
    println!("--- Interval Demo ---");
    println!("Creating an interval that fires every 50ms, 5 times...");

    let start = now();
    let tick_times: Arc<std::sync::Mutex<Vec<f64>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let tick_times_clone = tick_times.clone();

    let t = interval(
        move |elapsed| {
            tick_times_clone.lock().unwrap().push(elapsed);
            tick_times_clone.lock().unwrap().len() < 5
        },
        50.0, // 50ms interval
        None,
    );

    t.join();

    let total = now() - start;
    let times = tick_times.lock().unwrap();
    println!("Interval ticks at:");
    for (i, time) in times.iter().enumerate() {
        let expected = (i + 1) as f64 * 50.0;
        println!(
            "  Tick {}: {:.1}ms (expected ~{:.0}ms)",
            i + 1,
            time,
            expected
        );
    }
    println!("Total runtime: {:.1}ms\n", total);

    // Multiple concurrent timers demo
    println!("--- Concurrent Timers Demo ---");
    println!("Running 3 timers simultaneously...");

    let start = now();
    let counter1 = Arc::new(AtomicUsize::new(0));
    let counter2 = Arc::new(AtomicUsize::new(0));
    let counter3 = Arc::new(AtomicUsize::new(0));

    let c1 = counter1.clone();
    let c2 = counter2.clone();
    let c3 = counter3.clone();

    // Timer 1: Fast, stops after 3 ticks
    let t1 = timer(
        move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
            c1.load(Ordering::SeqCst) < 3
        },
        None,
        None,
    );

    // Timer 2: 30ms interval, stops after 4 ticks
    let t2 = Interval::new(
        move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
            c2.load(Ordering::SeqCst) < 4
        },
        30.0,
        None,
    );

    // Timer 3: 100ms timeout
    let t3 = Timeout::new(
        move |_| {
            c3.fetch_add(1, Ordering::SeqCst);
        },
        100.0,
        None,
    );

    // Wait for all to complete
    t1.join();
    t2.join();
    t3.join();

    let total = now() - start;
    println!(
        "Timer 1 ticks: {} (target: 3)",
        counter1.load(Ordering::SeqCst)
    );
    println!(
        "Timer 2 ticks: {} (target: 4)",
        counter2.load(Ordering::SeqCst)
    );
    println!(
        "Timer 3 ticks: {} (target: 1)",
        counter3.load(Ordering::SeqCst)
    );
    println!("All completed in {:.1}ms\n", total);

    // Timer restart demo
    println!("--- Timer Restart Demo ---");
    println!("Creating a timer, letting it run, then restarting...");

    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();

    let mut t = timer(
        move |_| {
            c.fetch_add(1, Ordering::SeqCst);
            c.load(Ordering::SeqCst) < 2
        },
        None,
        None,
    );

    // Wait for first timer to complete
    thread::sleep(Duration::from_millis(100));
    let first_count = counter.load(Ordering::SeqCst);
    println!("First timer completed with {} ticks", first_count);

    // Restart with new callback
    let counter2 = Arc::new(AtomicUsize::new(0));
    let c2 = counter2.clone();

    t.restart(
        move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
            c2.load(Ordering::SeqCst) < 3
        },
        None,
        None,
    );

    t.join();
    println!(
        "Restarted timer completed with {} ticks\n",
        counter2.load(Ordering::SeqCst)
    );

    // Timer properties demo
    println!("--- Timer Properties Demo ---");
    let t = timer(|_| false, Some(100.0), Some(1000.0));
    println!("Timer ID: {}", t.id());
    println!("Timer delay: {}ms", t.delay());
    println!("Timer start_time: {}ms", t.start_time());
    println!("Timer is_stopped (before join): {}", t.is_stopped());
    thread::sleep(Duration::from_millis(150));
    println!("Timer is_stopped (after callback): {}\n", t.is_stopped());

    println!("=== Demo Complete ===");
}
