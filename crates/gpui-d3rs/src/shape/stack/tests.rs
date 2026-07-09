use super::Stack;
use super::StackLayoutError;
use super::stack;
use super::stack_expand;
use super::streamgraph;
use super::try_stack;
use super::try_stack_expand;
use super::try_streamgraph;
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

#[test]
fn try_generate_matches_generate_for_finite_rectangular_data() {
    let data = vec![
        vec![1.0, 2.0, 3.0],
        vec![2.0, 3.0, 4.0],
        vec![3.0, 4.0, 5.0],
    ];
    let keys = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let stack = Stack::new()
        .keys(keys)
        .order(StackOrder::Descending)
        .offset(StackOffset::Expand);

    let permissive = stack.generate(&data);
    let checked = stack.try_generate(&data).unwrap();

    assert_eq!(checked.len(), permissive.len());
    for (checked, permissive) in checked.iter().zip(permissive.iter()) {
        assert_eq!(checked.key, permissive.key);
        assert_eq!(checked.data, permissive.data);
        assert_eq!(checked.values, permissive.values);
        assert_eq!(checked.index, permissive.index);
    }
}

#[test]
fn try_generate_rejects_ragged_rows() {
    let data = vec![vec![1.0, 2.0], vec![3.0]];
    let keys = vec!["A".to_string(), "B".to_string()];

    let error = Stack::new().keys(keys).try_generate(&data).unwrap_err();

    assert_eq!(
        error,
        StackLayoutError::RowLengthMismatch {
            row_index: 1,
            expected: 2,
            actual: 1,
        }
    );
}

#[test]
fn try_generate_rejects_non_finite_values() {
    let data = vec![vec![1.0, 2.0], vec![3.0, f64::NAN]];
    let keys = vec!["A".to_string(), "B".to_string()];

    let error = Stack::new().keys(keys).try_generate(&data).unwrap_err();

    assert_eq!(
        error,
        StackLayoutError::NonFiniteValue {
            row_index: 1,
            series_index: 1,
        }
    );
}

#[test]
fn checked_stack_helpers_validate_before_layout() {
    let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];

    assert_eq!(try_stack(&data).unwrap().len(), stack(&data).len());
    assert_eq!(
        try_stack_expand(&data).unwrap().len(),
        stack_expand(&data).len()
    );
    assert_eq!(
        try_streamgraph(&data).unwrap().len(),
        streamgraph(&data).len()
    );

    let ragged = vec![vec![1.0, 2.0], vec![3.0]];
    assert!(matches!(
        try_stack(&ragged),
        Err(StackLayoutError::RowLengthMismatch { .. })
    ));
}
