use super::Stack;
use super::stack_expand;
use super::streamgraph;
use super::types::StackOffset;
use super::types::StackOrder;

#[test]
fn test_stack_basic() {
    let data = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];

    let keys = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let result = Stack::new().keys(keys).generate(&data);

    assert_eq!(result.len(), 3);
}

#[test]
fn test_stack_values() {
    let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];

    let keys = vec!["A".to_string(), "B".to_string()];
    let result = Stack::new().keys(keys).generate(&data);

    // First series: [0, 1], [0, 3]
    // Second series: [1, 3], [3, 7]
    assert_eq!(result[0].values[0], [0.0, 1.0]);
    assert_eq!(result[0].values[1], [0.0, 3.0]);
}

#[test]
fn test_stack_expand() {
    let data = vec![vec![1.0, 1.0], vec![1.0, 1.0]];

    let result = stack_expand(&data);

    // Should normalize to [0, 1]
    let sum: f64 = result.iter().map(|s| s.values[0][1] - s.values[0][0]).sum();
    assert!((sum - 1.0).abs() < 0.001);
}

#[test]
fn test_stack_silhouette() {
    let data = vec![vec![1.0, 1.0]];

    let keys = vec!["A".to_string(), "B".to_string()];
    let result = Stack::new()
        .keys(keys)
        .offset(StackOffset::Silhouette)
        .generate(&data);

    // Should be centered around zero
    let mid = (result[0].values[0][0] + result.last().unwrap().values[0][1]) / 2.0;
    assert!(mid.abs() < 0.001);
}

#[test]
fn test_stack_order_descending() {
    let data = vec![vec![1.0, 3.0, 2.0]];

    let keys = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let result = Stack::new()
        .keys(keys)
        .order(StackOrder::Descending)
        .generate(&data);

    // Largest sum should be first
    assert!(result[0].key == "B");
}

#[test]
fn test_stack_order_preserves_data_values() {
    // Test that reordering uses correct data values, not reordered indices
    let data = vec![vec![10.0, 100.0, 1.0]]; // A=10, B=100, C=1

    let keys = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let result = Stack::new()
        .keys(keys)
        .order(StackOrder::Descending) // Order: B(100), A(10), C(1)
        .generate(&data);

    // After descending order: B first, then A, then C
    assert_eq!(result[0].key, "B");
    assert_eq!(result[1].key, "A");
    assert_eq!(result[2].key, "C");

    // Verify the stacked values use correct data
    // B: [0, 100]
    assert_eq!(result[0].values[0], [0.0, 100.0]);
    // A: [100, 110]
    assert_eq!(result[1].values[0], [100.0, 110.0]);
    // C: [110, 111]
    assert_eq!(result[2].values[0], [110.0, 111.0]);
}

#[test]
fn test_streamgraph() {
    let data = vec![
        vec![1.0, 2.0, 1.0],
        vec![2.0, 3.0, 2.0],
        vec![1.0, 2.0, 1.0],
    ];

    let result = streamgraph(&data);
    assert_eq!(result.len(), 3);
}

#[test]
fn test_stack_empty() {
    let data: Vec<Vec<f64>> = vec![];
    let result = Stack::new().keys(vec!["A".to_string()]).generate(&data);
    assert!(result.is_empty());
}
