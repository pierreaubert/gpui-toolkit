//! Transition module demonstration
//!
//! Run with: cargo run --example transition_demo --no-default-features

use d3rs::ease::{
    ease_back_in_out, ease_bounce_out, ease_cubic_in_out, ease_elastic_out, ease_linear,
};
use d3rs::transition::{Transition, TransitionManager, TransitionState};

fn main() {
    println!("=== d3-transition Demonstration ===\n");

    // =========================================================================
    // Basic Transition
    // =========================================================================

    println!("--- Basic Linear Transition ---\n");

    let mut transition = Transition::new()
        .duration(1000.0) // 1 second
        .from_to(0.0, 100.0);

    println!("Initial state: {:?}", transition.state());
    println!("Initial value: {:.2}", transition.value());

    // Simulate animation loop
    let frame_time = 16.67; // ~60fps
    let mut total_time = 0.0;

    while !transition.is_complete() && total_time < 1100.0 {
        let value = transition.tick(frame_time);
        total_time += frame_time;

        if total_time % 200.0 < frame_time {
            println!(
                "  t={:.0}ms: value={:.2}, state={:?}",
                total_time,
                value,
                transition.state()
            );
        }
    }

    println!("Final state: {:?}\n", transition.state());

    // =========================================================================
    // Transition with Delay
    // =========================================================================

    println!("--- Transition with 500ms Delay ---\n");

    let mut delayed = Transition::new()
        .duration(500.0)
        .delay(500.0)
        .from_to(0.0, 50.0);

    let mut total_time = 0.0;
    while !delayed.is_complete() && total_time < 1100.0 {
        let value = delayed.tick(frame_time);
        total_time += frame_time;

        if total_time % 200.0 < frame_time {
            println!(
                "  t={:.0}ms: value={:.2}, state={:?}",
                total_time,
                value,
                delayed.state()
            );
        }
    }

    println!();

    // =========================================================================
    // Different Easing Functions
    // =========================================================================

    println!("--- Easing Function Comparison ---\n");

    let easing_functions = [
        ("Linear", ease_linear as fn(f64) -> f64),
        ("Cubic In-Out", ease_cubic_in_out),
        ("Elastic Out", ease_elastic_out),
        ("Bounce Out", ease_bounce_out),
        ("Back In-Out", ease_back_in_out),
    ];

    for (name, ease_fn) in &easing_functions {
        let _trans = Transition::new()
            .duration(1000.0)
            .ease(*ease_fn)
            .from_to(0.0, 100.0);

        println!("{}:", name);

        // Sample at key points
        for &progress in &[0.0, 250.0, 500.0, 750.0, 1000.0] {
            let mut t = Transition::new()
                .duration(1000.0)
                .ease(*ease_fn)
                .from_to(0.0, 100.0);

            while t.state() != TransitionState::Ended {
                t.tick(16.67);
                let elapsed = match t.state() {
                    TransitionState::Pending => 0.0,
                    _ => {
                        // Approximate elapsed time
                        let val = t.value();
                        (val / 100.0) * 1000.0
                    }
                };

                if (elapsed - progress).abs() < 20.0 || t.is_complete() {
                    if (elapsed - progress).abs() < 20.0 {
                        println!("  t={:.0}ms -> {:.2}", progress, t.value());
                    }
                    break;
                }
            }
        }
        println!();
    }

    // =========================================================================
    // Lifecycle Callbacks
    // =========================================================================

    println!("--- Lifecycle Callbacks ---\n");

    let mut callback_demo = Transition::new()
        .duration(500.0)
        .from_to(0.0, 100.0)
        .on_start(|| println!("  -> Transition started!"))
        .on_end(|| println!("  -> Transition ended!"));

    let mut total_time = 0.0;
    while !callback_demo.is_complete() && total_time < 600.0 {
        callback_demo.tick(frame_time);
        total_time += frame_time;
    }

    println!();

    // =========================================================================
    // Interruption
    // =========================================================================

    println!("--- Transition Interruption ---\n");

    let mut interruptible = Transition::new()
        .duration(1000.0)
        .from_to(0.0, 100.0)
        .on_interrupt(|| println!("  -> Transition interrupted!"));

    // Run for 500ms then interrupt
    let mut total_time = 0.0;
    while total_time < 500.0 {
        let value = interruptible.tick(frame_time);
        total_time += frame_time;
        println!("  t={:.0}ms: value={:.2}", total_time, value);
    }

    println!("Interrupting at t=500ms...");
    interruptible.interrupt();
    println!("State after interrupt: {:?}\n", interruptible.state());

    // =========================================================================
    // TransitionManager - Multiple Named Transitions
    // =========================================================================

    println!("--- TransitionManager ---\n");

    let mut manager = TransitionManager::new();

    // Add multiple transitions
    manager.add(
        "opacity",
        Transition::new()
            .duration(1000.0)
            .ease(ease_cubic_in_out)
            .from_to(0.0, 1.0),
    );

    manager.add(
        "x",
        Transition::new()
            .duration(1000.0)
            .ease(ease_back_in_out)
            .from_to(0.0, 500.0),
    );

    manager.add(
        "y",
        Transition::new()
            .duration(800.0)
            .delay(200.0)
            .ease(ease_elastic_out)
            .from_to(0.0, 300.0),
    );

    println!("Running 3 transitions simultaneously:\n");

    let mut total_time = 0.0;
    while manager.is_animating() && total_time < 1200.0 {
        let values = manager.tick(frame_time);
        total_time += frame_time;

        if total_time % 200.0 < frame_time {
            println!("t={:.0}ms:", total_time);
            for (name, value) in &values {
                println!("  {}: {:.2}", name, value);
            }
        }
    }

    println!("\nAll transitions complete!\n");

    // =========================================================================
    // Transition Replacement
    // =========================================================================

    println!("--- Transition Replacement ---\n");

    let mut manager = TransitionManager::new();

    manager.add(
        "position",
        Transition::new()
            .duration(1000.0)
            .from_to(0.0, 100.0)
            .on_interrupt(|| println!("  -> First transition interrupted")),
    );

    // Run for 300ms
    for _ in 0..18 {
        manager.tick(frame_time);
    }

    let pos = manager.get("position").unwrap();
    println!("Position after 300ms: {:.2}", pos);

    // Replace with new transition
    println!("Replacing with new transition...");
    manager.add(
        "position",
        Transition::new()
            .duration(500.0)
            .from_to(pos, 200.0)
            .on_start(|| println!("  -> New transition started")),
    );

    // Run to completion
    while manager.is_animating() {
        manager.tick(frame_time);
    }

    println!(
        "Final position: {:.2}\n",
        manager.get("position").unwrap_or(0.0)
    );

    // =========================================================================
    // Chained Transitions (sequential)
    // =========================================================================

    println!("--- Chained Transitions ---\n");

    let mut manager = TransitionManager::new();

    // First transition: 0 -> 100
    manager.add(
        "value",
        Transition::new()
            .duration(500.0)
            .from_to(0.0, 100.0)
            .on_end(|| println!("  Phase 1 complete: reached 100")),
    );

    // Run first transition
    while manager.is_animating() {
        manager.tick(frame_time);
    }

    // Second transition: 100 -> 50 (with delay)
    manager.add(
        "value",
        Transition::new()
            .duration(500.0)
            .delay(200.0)
            .from_to(100.0, 50.0)
            .on_end(|| println!("  Phase 2 complete: reached 50")),
    );

    // Run second transition
    while manager.is_animating() {
        manager.tick(frame_time);
    }

    let final_value = manager.get("value").unwrap_or(0.0);
    println!("Final value: {:.2}\n", final_value);

    // =========================================================================
    // Summary
    // =========================================================================

    println!("=== Demo Complete ===\n");
    println!("Key Features Demonstrated:");
    println!("  ✓ Basic transitions with duration control");
    println!("  ✓ Delayed transitions");
    println!("  ✓ Multiple easing functions");
    println!("  ✓ Lifecycle callbacks (on_start, on_end, on_interrupt)");
    println!("  ✓ Transition interruption");
    println!("  ✓ Multiple simultaneous transitions via TransitionManager");
    println!("  ✓ Named transition replacement");
    println!("  ✓ Chained/sequential transitions");
}
