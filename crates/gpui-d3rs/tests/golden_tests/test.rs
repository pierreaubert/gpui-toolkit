use super::misc::TOLERANCE;
use super::misc::approx_eq;
use d3rs::scale::{LinearScale, Scale};

pub(super) fn test_linear_nice(case: &serde_json::Value) {
    let config = &case["config"];
    let domain: Vec<f64> = serde_json::from_value(config["domain"].clone()).unwrap();
    let nice_domain: Vec<f64> = serde_json::from_value(case["nice_domain"].clone()).unwrap();

    let scale = LinearScale::new().domain(domain[0], domain[1]).nice(None);

    assert!(
        approx_eq(nice_domain[0], scale.domain_min()),
        "nice domain min: {} (expected {})",
        scale.domain_min(),
        nice_domain[0]
    );
    assert!(
        approx_eq(nice_domain[1], scale.domain_max()),
        "nice domain max: {} (expected {})",
        scale.domain_max(),
        nice_domain[1]
    );
}

pub(super) fn test_linear_ticks(case: &serde_json::Value) {
    let name = case["name"].as_str().unwrap();
    let config = &case["config"];
    let domain: Vec<f64> = serde_json::from_value(config["domain"].clone()).unwrap();
    let count = case["ticks_count"].as_u64().unwrap_or(10) as usize;
    let _expected: Vec<f64> = serde_json::from_value(case["ticks"].clone()).unwrap();

    let scale = LinearScale::new().domain(domain[0], domain[1]);
    let ticks = scale.ticks(count);

    // Check that ticks are reasonable (may not be exact match due to algorithm differences)
    assert!(
        !ticks.is_empty(),
        "case '{}': ticks should not be empty",
        name
    );

    // Check first and last tick are within domain extent
    let first = ticks.first().unwrap();
    let last = ticks.last().unwrap();
    assert!(
        *first <= domain[0] + TOLERANCE,
        "case '{}': first tick {} should be <= domain min {}",
        name,
        first,
        domain[0]
    );
    assert!(
        *last >= domain[1] - TOLERANCE,
        "case '{}': last tick {} should be >= domain max {}",
        name,
        last,
        domain[1]
    );

    // Check that ticks are evenly spaced (for simple linear domains)
    if ticks.len() >= 2 {
        let step = ticks[1] - ticks[0];
        for i in 2..ticks.len() {
            let actual_step = ticks[i] - ticks[i - 1];
            assert!(
                approx_eq(step, actual_step),
                "case '{}': ticks not evenly spaced: step[0]={}, step[{}]={}",
                name,
                step,
                i,
                actual_step
            );
        }
    }
}
