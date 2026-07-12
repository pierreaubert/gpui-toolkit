//! Benchmark coverage report for release QA.

use serde::Serialize;

/// Schema version for [`BenchmarkReport`].
pub const BENCHMARK_REPORT_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`BenchmarkReport`].
pub const BENCHMARK_REPORT_TYPE: &str = "gpui-builder-benchmark-coverage";

/// One Criterion benchmark case covered by the crate's solver benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BenchmarkCase {
    pub group: &'static str,
    pub id: &'static str,
    pub operation: &'static str,
    pub scale: &'static str,
    pub purpose: &'static str,
}

/// Versioned benchmark coverage report for release notes and CI logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BenchmarkReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub criterion_command: &'static str,
    pub baseline_policy: &'static str,
    pub cases: &'static [BenchmarkCase],
}

impl BenchmarkReport {
    /// Render the benchmark coverage report as Markdown.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# gpui-builder Benchmark Coverage\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - criterion_command: `{}`\n\
             - baseline_policy: {}\n\n\
             | Group | Case | Operation | Scale | Purpose |\n\
             | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.criterion_command, self.baseline_policy
        );

        for case in self.cases {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                case.group, case.id, case.operation, case.scale, case.purpose
            ));
        }

        markdown
    }
}

const BENCHMARK_CASES: [BenchmarkCase; 27] = [
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "recursive_find_depth_6",
        operation: "SolvedNode::find",
        scale: "balanced depth 6",
        purpose: "recursive lookup baseline on medium nested layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "as_map_get_depth_6",
        operation: "SolvedTree::as_map().get",
        scale: "balanced depth 6",
        purpose: "map lookup baseline on medium nested layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "flat_find_depth_6",
        operation: "SolvedTree::find",
        scale: "balanced depth 6",
        purpose: "flat-tree lookup baseline on medium nested layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "recursive_find_depth_8",
        operation: "SolvedNode::find",
        scale: "balanced depth 8",
        purpose: "recursive lookup scaling on large nested layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "as_map_get_depth_8",
        operation: "SolvedTree::as_map().get",
        scale: "balanced depth 8",
        purpose: "map lookup scaling on large nested layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "flat_find_depth_8",
        operation: "SolvedTree::find",
        scale: "balanced depth 8",
        purpose: "flat-tree lookup scaling on large nested layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "recursive_find_depth_10",
        operation: "SolvedNode::find",
        scale: "balanced depth 10",
        purpose: "recursive lookup stress case for deeply nested layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "as_map_get_depth_10",
        operation: "SolvedTree::as_map().get",
        scale: "balanced depth 10",
        purpose: "map lookup stress case for deeply nested layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_find",
        id: "flat_find_depth_10",
        operation: "SolvedTree::find",
        scale: "balanced depth 10",
        purpose: "flat-tree lookup stress case for deeply nested layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "recursive_find_count_50",
        operation: "SolvedNode::find",
        scale: "50 sibling slots",
        purpose: "recursive lookup baseline on wide layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "as_map_get_count_50",
        operation: "SolvedTree::as_map().get",
        scale: "50 sibling slots",
        purpose: "map lookup baseline on wide layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "flat_find_count_50",
        operation: "SolvedTree::find",
        scale: "50 sibling slots",
        purpose: "flat-tree lookup baseline on wide layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "recursive_find_count_200",
        operation: "SolvedNode::find",
        scale: "200 sibling slots",
        purpose: "recursive lookup scaling on wide layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "as_map_get_count_200",
        operation: "SolvedTree::as_map().get",
        scale: "200 sibling slots",
        purpose: "map lookup scaling on wide layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "flat_find_count_200",
        operation: "SolvedTree::find",
        scale: "200 sibling slots",
        purpose: "flat-tree lookup scaling on wide layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "recursive_find_count_500",
        operation: "SolvedNode::find",
        scale: "500 sibling slots",
        purpose: "recursive lookup stress case on wide layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "as_map_get_count_500",
        operation: "SolvedTree::as_map().get",
        scale: "500 sibling slots",
        purpose: "map lookup stress case on wide layouts",
    },
    BenchmarkCase {
        group: "wide_tree_find",
        id: "flat_find_count_500",
        operation: "SolvedTree::find",
        scale: "500 sibling slots",
        purpose: "flat-tree lookup stress case on wide layouts",
    },
    BenchmarkCase {
        group: "balanced_tree_traversal",
        id: "recursive_collect_depth_6",
        operation: "recursive solved-node traversal",
        scale: "balanced depth 6",
        purpose: "recursive traversal baseline",
    },
    BenchmarkCase {
        group: "balanced_tree_traversal",
        id: "flat_iter_depth_6",
        operation: "SolvedTree::iter",
        scale: "balanced depth 6",
        purpose: "flat traversal baseline",
    },
    BenchmarkCase {
        group: "balanced_tree_traversal",
        id: "recursive_collect_depth_8",
        operation: "recursive solved-node traversal",
        scale: "balanced depth 8",
        purpose: "recursive traversal scaling",
    },
    BenchmarkCase {
        group: "balanced_tree_traversal",
        id: "flat_iter_depth_8",
        operation: "SolvedTree::iter",
        scale: "balanced depth 8",
        purpose: "flat traversal scaling",
    },
    BenchmarkCase {
        group: "balanced_tree_traversal",
        id: "recursive_collect_depth_10",
        operation: "recursive solved-node traversal",
        scale: "balanced depth 10",
        purpose: "recursive traversal stress case",
    },
    BenchmarkCase {
        group: "balanced_tree_traversal",
        id: "flat_iter_depth_10",
        operation: "SolvedTree::iter",
        scale: "balanced depth 10",
        purpose: "flat traversal stress case",
    },
    BenchmarkCase {
        group: "text_cache_hit",
        id: "solve_text_cache_hit",
        operation: "solve",
        scale: "20 text-measured slots",
        purpose: "recursive solver text-measure cache-hit baseline",
    },
    BenchmarkCase {
        group: "text_cache_hit",
        id: "solve_tree_text_cache_hit",
        operation: "solve_tree",
        scale: "20 text-measured slots",
        purpose: "flat solver text-measure cache-hit baseline",
    },
    BenchmarkCase {
        group: "text_cache_hit",
        id: "solve_tree_into_text_cache_hit",
        operation: "solve_tree_into",
        scale: "20 text-measured slots",
        purpose: "reusable flat solver cache-hit and resize hot-path baseline",
    },
];

/// Return the current benchmark coverage report.
pub fn benchmark_report() -> BenchmarkReport {
    BenchmarkReport {
        schema_version: BENCHMARK_REPORT_SCHEMA_VERSION,
        report_type: BENCHMARK_REPORT_TYPE,
        criterion_command: "cargo bench -p gpui-builder --bench solved_tree",
        baseline_policy: "Record Criterion estimates on the release machine before publishing and compare the same case ids across releases.",
        cases: &BENCHMARK_CASES,
    }
}

/// Return benchmark cases without allocating.
pub fn benchmark_cases() -> &'static [BenchmarkCase] {
    &BENCHMARK_CASES
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn benchmark_report_has_stable_contract() {
        let report = benchmark_report();

        assert_eq!(report.schema_version, BENCHMARK_REPORT_SCHEMA_VERSION);
        assert_eq!(report.report_type, BENCHMARK_REPORT_TYPE);
        assert!(report.criterion_command.contains("solved_tree"));
        assert_eq!(report.cases.len(), 27);

        for case in report.cases {
            assert!(!case.group.is_empty());
            assert!(!case.id.is_empty());
            assert!(!case.operation.is_empty());
            assert!(!case.scale.is_empty());
            assert!(!case.purpose.is_empty());
        }
    }

    #[test]
    fn benchmark_report_covers_all_solver_hot_paths() {
        let groups = benchmark_report()
            .cases
            .iter()
            .map(|case| case.group)
            .collect::<BTreeSet<_>>();

        assert!(groups.contains("balanced_tree_find"));
        assert!(groups.contains("wide_tree_find"));
        assert!(groups.contains("balanced_tree_traversal"));
        assert!(groups.contains("text_cache_hit"));
    }

    #[test]
    fn benchmark_report_has_unique_case_ids() {
        let mut ids = BTreeSet::new();
        for case in benchmark_report().cases {
            assert!(ids.insert(case.id), "duplicate benchmark id {}", case.id);
        }
    }

    #[test]
    fn benchmark_report_markdown_names_command_and_cases() {
        let markdown = benchmark_report().to_markdown_table();

        assert!(markdown.contains(BENCHMARK_REPORT_TYPE));
        assert!(markdown.contains("cargo bench -p gpui-builder --bench solved_tree"));
        assert!(markdown.contains("recursive_find_depth_10"));
        assert!(markdown.contains("solve_tree_text_cache_hit"));
        assert!(markdown.contains("solve_tree_into_text_cache_hit"));
    }

    #[test]
    fn benchmark_report_serializes_for_ci_artifacts() {
        let json = serde_json::to_string(&benchmark_report()).unwrap();

        assert!(json.contains(BENCHMARK_REPORT_TYPE));
        assert!(json.contains("balanced_tree_find"));
        assert!(json.contains("text_cache_hit"));
    }
}
