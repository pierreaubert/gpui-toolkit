//! d3-array demonstration
//!
//! This example demonstrates the array utilities inspired by d3-array:
//! - Statistics (mean, median, deviation, quantile)
//! - Search (bisect, binary search)
//! - Transformations (group, rollup, bin)
//! - Ticks generation

use d3rs::array::{
    Bisector,
    bin,
    // Search
    bisect_left_f64,
    bisect_right_f64,
    cumsum,
    deviation,
    // Set operations
    difference,
    extent_by,
    // Transformations
    group,
    intersection,
    log_ticks,
    max_by,
    max_index,
    // Statistics
    mean,
    median,
    min_by,
    min_index,
    nice,
    quantile,
    rollup,
    sum,
    tick_step,
    // Ticks
    ticks,
    union,
    variance,
};
use std::cmp::Ordering;

/// Comparator for f64 values
fn f64_cmp(a: &f64, b: &f64) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Equal)
}

fn main() {
    println!("=== d3-array Demonstration ===\n");

    // Sample data
    let data = vec![4.0, 2.0, 7.0, 1.0, 9.0, 3.0, 6.0, 8.0, 5.0];
    let mut data_for_median = data.clone();
    println!("Sample data: {:?}\n", data);

    // ========================================
    // Statistics
    // ========================================
    println!("--- Statistics ---");

    println!("sum:      {:?}", sum(&data));
    println!("mean:     {:?}", mean(&data));
    println!("median:   {:?}", median(&mut data_for_median));
    println!("variance: {:?}", variance(&data));
    println!("deviation:{:?}", deviation(&data));
    println!("extent:   {:?}", extent_by(&data, f64_cmp));
    println!("min:      {:?}", min_by(&data, f64_cmp));
    println!("max:      {:?}", max_by(&data, f64_cmp));

    // min_index and max_index work differently - they're for Ord types
    let int_data: Vec<i32> = data.iter().map(|x| *x as i32).collect();
    println!("min_index:{:?}", min_index(&int_data));
    println!("max_index:{:?}", max_index(&int_data));

    // Quantiles
    let mut data_for_quantile = data.clone();
    println!("\nQuantiles:");
    println!("  Q1 (25%): {:?}", quantile(&mut data_for_quantile, 0.25));
    println!("  Q2 (50%): {:?}", quantile(&mut data_for_quantile, 0.50));
    println!("  Q3 (75%): {:?}", quantile(&mut data_for_quantile, 0.75));

    // Cumulative sum
    println!("\nCumulative sum: {:?}", cumsum(&data));

    // ========================================
    // Search / Bisection
    // ========================================
    println!("\n--- Search / Bisection ---");

    let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    println!("Sorted array: {:?}", sorted);

    // Find insertion points (using f64-specific functions)
    println!(
        "bisect_left(4.5):  {} (insert between 4 and 5)",
        bisect_left_f64(&sorted, 4.5)
    );
    println!(
        "bisect_left(5.0):  {} (insert before 5)",
        bisect_left_f64(&sorted, 5.0)
    );
    println!(
        "bisect_right(5.0): {} (insert after 5)",
        bisect_right_f64(&sorted, 5.0)
    );

    // Custom bisector for objects
    #[derive(Debug)]
    struct Person {
        name: &'static str,
        age: u32,
    }

    let people = vec![
        Person {
            name: "Alice",
            age: 25,
        },
        Person {
            name: "Bob",
            age: 30,
        },
        Person {
            name: "Carol",
            age: 35,
        },
        Person {
            name: "Dave",
            age: 40,
        },
    ];

    let age_bisector = Bisector::new(|p: &Person| p.age as f64);
    println!(
        "\nPeople by age: {:?}",
        people.iter().map(|p| (p.name, p.age)).collect::<Vec<_>>()
    );
    println!(
        "bisect(age=32): {} (Carol is at 35)",
        age_bisector.right(&people, 32.0)
    );

    // ========================================
    // Grouping and Rollup
    // ========================================
    println!("\n--- Grouping and Rollup ---");

    #[derive(Debug, Clone)]
    struct Sale {
        category: String,
        amount: f64,
    }

    let sales = vec![
        Sale {
            category: "Electronics".into(),
            amount: 100.0,
        },
        Sale {
            category: "Books".into(),
            amount: 25.0,
        },
        Sale {
            category: "Electronics".into(),
            amount: 200.0,
        },
        Sale {
            category: "Books".into(),
            amount: 15.0,
        },
        Sale {
            category: "Clothing".into(),
            amount: 75.0,
        },
    ];

    // Group by category
    let grouped = group(&sales, |s| s.category.clone());
    println!("Grouped by category:");
    for (category, items) in &grouped {
        println!("  {}: {} items", category, items.len());
    }

    // Rollup - sum amounts by category
    let totals = rollup(
        &sales,
        |s| s.category.clone(),
        |items| items.iter().map(|s| s.amount).sum::<f64>(),
    );
    println!("\nTotal by category:");
    for (category, total) in &totals {
        println!("  {}: ${:.2}", category, total);
    }

    // ========================================
    // Binning (Histograms)
    // ========================================
    println!("\n--- Binning (Histograms) ---");

    let values: Vec<f64> = vec![
        12.0, 15.0, 18.0, 22.0, 25.0, 28.0, 31.0, 35.0, 42.0, 45.0, 48.0, 55.0, 62.0, 68.0, 75.0,
        82.0,
    ];
    println!("Values: {:?}", values);

    // Create bins using the bin function (5 bins)
    let bins = bin(&values, 5);

    println!("\nHistogram (5 bins):");
    for bin in &bins {
        let bar: String = std::iter::repeat('#').take(bin.values.len() * 2).collect();
        println!(
            "  [{:5.1}, {:5.1}): {} {}",
            bin.x0,
            bin.x1,
            bar,
            bin.values.len()
        );
    }

    // ========================================
    // Ticks Generation
    // ========================================
    println!("\n--- Ticks Generation ---");

    // Linear ticks
    let linear_ticks = ticks(0.0, 100.0, 10);
    println!("Linear ticks (0-100, ~10 ticks): {:?}", linear_ticks);

    // Tick step
    let step = tick_step(0.0, 100.0, 10);
    println!("Tick step: {}", step);

    // Nice domain
    let (nice_start, nice_end) = nice(0.127, 9.873, 10);
    println!("Nice domain (0.127, 9.873): ({}, {})", nice_start, nice_end);

    // Logarithmic ticks (base 10, no subdivisions)
    let log_t = log_ticks(1.0, 1000.0, 10.0, false);
    println!("Log ticks (1-1000, base 10): {:?}", log_t);

    // ========================================
    // Set Operations
    // ========================================
    println!("\n--- Set Operations ---");

    let set_a = vec![1, 2, 3, 4, 5];
    let set_b = vec![3, 4, 5, 6, 7];

    println!("Set A: {:?}", set_a);
    println!("Set B: {:?}", set_b);
    println!("Union:        {:?}", union(&set_a, &set_b));
    println!("Intersection: {:?}", intersection(&set_a, &set_b));
    println!("Difference:   {:?}", difference(&set_a, &set_b));

    println!("\n=== End of d3-array Demo ===");
}
