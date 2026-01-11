//! Flare.js Hierarchy Dataset
//!
//! This dataset represents the package structure of the Flare visualization library,
//! with byte sizes for each component.
//!
//! # Data Source
//! - Original: Flare visualization toolkit (http://flare.prefuse.org/)
//! - D3 version: https://github.com/d3/d3-hierarchy/blob/main/test/data/flare.json
//!
//! # License
//! The Flare library is licensed under the BSD license.
//! This data representation (file sizes/structure) is factual and non-copyrightable.

/// A node in the hierarchy tree
#[derive(Debug, Clone)]
pub struct HierarchyNode {
    pub name: String,
    pub value: Option<u64>,
    pub children: Vec<HierarchyNode>,
}

impl HierarchyNode {
    pub fn leaf(name: &str, value: u64) -> Self {
        Self {
            name: name.to_string(),
            value: Some(value),
            children: Vec::new(),
        }
    }

    pub fn branch(name: &str, children: Vec<HierarchyNode>) -> Self {
        Self {
            name: name.to_string(),
            value: None,
            children,
        }
    }

    /// Calculate the total value of this node and all descendants
    pub fn total_value(&self) -> u64 {
        if let Some(v) = self.value {
            v
        } else {
            self.children.iter().map(|c| c.total_value()).sum()
        }
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Get the depth of the tree
    #[allow(dead_code)]
    pub fn depth(&self) -> usize {
        if self.children.is_empty() {
            0
        } else {
            1 + self.children.iter().map(|c| c.depth()).max().unwrap_or(0)
        }
    }

    /// Count total number of leaf nodes
    #[allow(dead_code)]
    pub fn leaf_count(&self) -> usize {
        if self.is_leaf() {
            1
        } else {
            self.children.iter().map(|c| c.leaf_count()).sum()
        }
    }
}

/// Build the flare hierarchy dataset
pub fn flare_hierarchy() -> HierarchyNode {
    HierarchyNode::branch(
        "flare",
        vec![
            // analytics
            HierarchyNode::branch(
                "analytics",
                vec![
                    HierarchyNode::branch(
                        "cluster",
                        vec![
                            HierarchyNode::leaf("AgglomerativeCluster", 3938),
                            HierarchyNode::leaf("CommunityStructure", 3812),
                            HierarchyNode::leaf("HierarchicalCluster", 6714),
                            HierarchyNode::leaf("MergeEdge", 743),
                        ],
                    ),
                    HierarchyNode::branch(
                        "graph",
                        vec![
                            HierarchyNode::leaf("BetweennessCentrality", 3534),
                            HierarchyNode::leaf("LinkDistance", 5731),
                            HierarchyNode::leaf("MaxFlowMinCut", 7840),
                            HierarchyNode::leaf("ShortestPaths", 5914),
                            HierarchyNode::leaf("SpanningTree", 3416),
                        ],
                    ),
                    HierarchyNode::branch(
                        "optimization",
                        vec![HierarchyNode::leaf("AspectRatioBanker", 7074)],
                    ),
                ],
            ),
            // animate
            HierarchyNode::branch(
                "animate",
                vec![
                    HierarchyNode::leaf("Easing", 17010),
                    HierarchyNode::leaf("FunctionSequence", 5842),
                    HierarchyNode::branch(
                        "interpolate",
                        vec![
                            HierarchyNode::leaf("ArrayInterpolator", 1983),
                            HierarchyNode::leaf("ColorInterpolator", 2047),
                            HierarchyNode::leaf("DateInterpolator", 1375),
                            HierarchyNode::leaf("Interpolator", 8746),
                            HierarchyNode::leaf("MatrixInterpolator", 2202),
                            HierarchyNode::leaf("NumberInterpolator", 1382),
                            HierarchyNode::leaf("ObjectInterpolator", 1629),
                            HierarchyNode::leaf("PointInterpolator", 1675),
                            HierarchyNode::leaf("RectangleInterpolator", 2042),
                        ],
                    ),
                    HierarchyNode::leaf("ISchedulable", 1041),
                    HierarchyNode::leaf("Parallel", 5176),
                    HierarchyNode::leaf("Pause", 449),
                    HierarchyNode::leaf("Scheduler", 5593),
                    HierarchyNode::leaf("Sequence", 5534),
                    HierarchyNode::leaf("Transition", 9201),
                    HierarchyNode::leaf("Transitioner", 19975),
                    HierarchyNode::leaf("TransitionEvent", 1116),
                    HierarchyNode::leaf("Tween", 6006),
                ],
            ),
            // data
            HierarchyNode::branch(
                "data",
                vec![
                    HierarchyNode::branch(
                        "converters",
                        vec![
                            HierarchyNode::leaf("Converters", 721),
                            HierarchyNode::leaf("DelimitedTextConverter", 4294),
                            HierarchyNode::leaf("GraphMLConverter", 9800),
                            HierarchyNode::leaf("IDataConverter", 1314),
                            HierarchyNode::leaf("JSONConverter", 2220),
                        ],
                    ),
                    HierarchyNode::leaf("DataField", 1759),
                    HierarchyNode::leaf("DataSchema", 2165),
                    HierarchyNode::leaf("DataSet", 586),
                    HierarchyNode::leaf("DataSource", 3331),
                    HierarchyNode::leaf("DataTable", 772),
                    HierarchyNode::leaf("DataUtil", 3322),
                ],
            ),
            // display
            HierarchyNode::branch(
                "display",
                vec![
                    HierarchyNode::leaf("DirtySprite", 8833),
                    HierarchyNode::leaf("LineSprite", 1732),
                    HierarchyNode::leaf("RectSprite", 3623),
                    HierarchyNode::leaf("TextSprite", 10066),
                ],
            ),
            // flex
            HierarchyNode::branch("flex", vec![HierarchyNode::leaf("FlareVis", 4116)]),
            // physics
            HierarchyNode::branch(
                "physics",
                vec![
                    HierarchyNode::leaf("DragForce", 1082),
                    HierarchyNode::leaf("GravityForce", 1336),
                    HierarchyNode::leaf("IForce", 319),
                    HierarchyNode::leaf("NBodyForce", 10498),
                    HierarchyNode::leaf("Particle", 2822),
                    HierarchyNode::leaf("Simulation", 9983),
                    HierarchyNode::leaf("Spring", 2213),
                    HierarchyNode::leaf("SpringForce", 1681),
                ],
            ),
            // query
            HierarchyNode::branch(
                "query",
                vec![
                    HierarchyNode::leaf("AggregateExpression", 1616),
                    HierarchyNode::leaf("And", 1027),
                    HierarchyNode::leaf("Arithmetic", 3891),
                    HierarchyNode::leaf("Average", 891),
                    HierarchyNode::leaf("BinaryExpression", 2893),
                    HierarchyNode::leaf("Comparison", 5103),
                    HierarchyNode::leaf("CompositeExpression", 3677),
                    HierarchyNode::leaf("Count", 781),
                    HierarchyNode::leaf("DateUtil", 4141),
                    HierarchyNode::leaf("Distinct", 933),
                    HierarchyNode::leaf("Expression", 5130),
                    HierarchyNode::leaf("ExpressionIterator", 3617),
                    HierarchyNode::leaf("Fn", 3240),
                    HierarchyNode::leaf("If", 2732),
                    HierarchyNode::leaf("IsA", 2039),
                    HierarchyNode::leaf("Literal", 1214),
                    HierarchyNode::leaf("Match", 3748),
                    HierarchyNode::leaf("Maximum", 843),
                    HierarchyNode::branch(
                        "methods",
                        vec![
                            HierarchyNode::leaf("add", 593),
                            HierarchyNode::leaf("and", 330),
                            HierarchyNode::leaf("average", 287),
                            HierarchyNode::leaf("count", 277),
                            HierarchyNode::leaf("distinct", 292),
                            HierarchyNode::leaf("div", 595),
                            HierarchyNode::leaf("eq", 594),
                            HierarchyNode::leaf("fn", 460),
                            HierarchyNode::leaf("gt", 603),
                            HierarchyNode::leaf("gte", 625),
                            HierarchyNode::leaf("iff", 748),
                            HierarchyNode::leaf("isa", 461),
                            HierarchyNode::leaf("lt", 597),
                            HierarchyNode::leaf("lte", 619),
                            HierarchyNode::leaf("max", 283),
                            HierarchyNode::leaf("min", 283),
                            HierarchyNode::leaf("mod", 591),
                            HierarchyNode::leaf("mul", 603),
                            HierarchyNode::leaf("neq", 599),
                            HierarchyNode::leaf("not", 386),
                            HierarchyNode::leaf("or", 323),
                            HierarchyNode::leaf("orderby", 307),
                            HierarchyNode::leaf("range", 772),
                            HierarchyNode::leaf("select", 296),
                            HierarchyNode::leaf("stddev", 363),
                            HierarchyNode::leaf("sub", 600),
                            HierarchyNode::leaf("sum", 280),
                            HierarchyNode::leaf("update", 307),
                            HierarchyNode::leaf("variance", 335),
                            HierarchyNode::leaf("where", 299),
                            HierarchyNode::leaf("xor", 354),
                            HierarchyNode::leaf("_", 264),
                        ],
                    ),
                    HierarchyNode::leaf("Minimum", 843),
                    HierarchyNode::leaf("Not", 1554),
                    HierarchyNode::leaf("Or", 970),
                    HierarchyNode::leaf("Query", 13896),
                    HierarchyNode::leaf("Range", 1594),
                    HierarchyNode::leaf("StringUtil", 4130),
                    HierarchyNode::leaf("Sum", 791),
                    HierarchyNode::leaf("Variable", 1124),
                    HierarchyNode::leaf("Variance", 1876),
                    HierarchyNode::leaf("Xor", 1101),
                ],
            ),
            // scale
            HierarchyNode::branch(
                "scale",
                vec![
                    HierarchyNode::leaf("IScaleMap", 2105),
                    HierarchyNode::leaf("LinearScale", 1316),
                    HierarchyNode::leaf("LogScale", 3151),
                    HierarchyNode::leaf("OrdinalScale", 3770),
                    HierarchyNode::leaf("QuantileScale", 2435),
                    HierarchyNode::leaf("QuantitativeScale", 4839),
                    HierarchyNode::leaf("RootScale", 1756),
                    HierarchyNode::leaf("Scale", 4268),
                    HierarchyNode::leaf("ScaleType", 1821),
                    HierarchyNode::leaf("TimeScale", 5833),
                ],
            ),
            // util
            HierarchyNode::branch(
                "util",
                vec![
                    HierarchyNode::leaf("Arrays", 8258),
                    HierarchyNode::leaf("Colors", 10001),
                    HierarchyNode::leaf("Dates", 8217),
                    HierarchyNode::leaf("Displays", 12555),
                    HierarchyNode::leaf("Filter", 2324),
                    HierarchyNode::leaf("Geometry", 10993),
                    HierarchyNode::branch(
                        "heap",
                        vec![
                            HierarchyNode::leaf("FibonacciHeap", 9354),
                            HierarchyNode::leaf("HeapNode", 1233),
                        ],
                    ),
                    HierarchyNode::leaf("IEvaluable", 335),
                    HierarchyNode::leaf("IPredicate", 383),
                    HierarchyNode::leaf("IValueProxy", 874),
                    HierarchyNode::branch(
                        "math",
                        vec![
                            HierarchyNode::leaf("DenseMatrix", 3165),
                            HierarchyNode::leaf("IMatrix", 2815),
                            HierarchyNode::leaf("SparseMatrix", 3366),
                        ],
                    ),
                    HierarchyNode::leaf("Maths", 17705),
                    HierarchyNode::leaf("Orientation", 1486),
                    HierarchyNode::branch(
                        "palette",
                        vec![
                            HierarchyNode::leaf("ColorPalette", 6367),
                            HierarchyNode::leaf("Palette", 1229),
                            HierarchyNode::leaf("ShapePalette", 2059),
                            HierarchyNode::leaf("SizePalette", 2291),
                        ],
                    ),
                    HierarchyNode::leaf("Property", 5559),
                    HierarchyNode::leaf("Shapes", 19118),
                    HierarchyNode::leaf("Sort", 6887),
                    HierarchyNode::leaf("Stats", 6557),
                    HierarchyNode::leaf("Strings", 22026),
                ],
            ),
            // vis
            HierarchyNode::branch(
                "vis",
                vec![
                    HierarchyNode::branch(
                        "axis",
                        vec![
                            HierarchyNode::leaf("Axes", 1302),
                            HierarchyNode::leaf("Axis", 24593),
                            HierarchyNode::leaf("AxisGridLine", 652),
                            HierarchyNode::leaf("AxisLabel", 636),
                            HierarchyNode::leaf("CartesianAxes", 6703),
                        ],
                    ),
                    HierarchyNode::branch(
                        "controls",
                        vec![
                            HierarchyNode::leaf("AnchorControl", 2138),
                            HierarchyNode::leaf("ClickControl", 3824),
                            HierarchyNode::leaf("Control", 1353),
                            HierarchyNode::leaf("ControlList", 4665),
                            HierarchyNode::leaf("DragControl", 2649),
                            HierarchyNode::leaf("ExpandControl", 2832),
                            HierarchyNode::leaf("HoverControl", 4896),
                            HierarchyNode::leaf("IControl", 763),
                            HierarchyNode::leaf("PanZoomControl", 5222),
                            HierarchyNode::leaf("SelectionControl", 7862),
                            HierarchyNode::leaf("TooltipControl", 8435),
                        ],
                    ),
                    HierarchyNode::branch(
                        "data",
                        vec![
                            HierarchyNode::leaf("Data", 20544),
                            HierarchyNode::leaf("DataList", 19788),
                            HierarchyNode::leaf("DataSprite", 10349),
                            HierarchyNode::leaf("EdgeSprite", 3301),
                            HierarchyNode::leaf("NodeSprite", 19382),
                            HierarchyNode::branch(
                                "render",
                                vec![
                                    HierarchyNode::leaf("ArrowType", 698),
                                    HierarchyNode::leaf("EdgeRenderer", 5569),
                                    HierarchyNode::leaf("IRenderer", 353),
                                    HierarchyNode::leaf("ShapeRenderer", 2247),
                                ],
                            ),
                            HierarchyNode::leaf("ScaleBinding", 11275),
                            HierarchyNode::leaf("Tree", 7147),
                            HierarchyNode::leaf("TreeBuilder", 9930),
                        ],
                    ),
                    HierarchyNode::branch(
                        "events",
                        vec![
                            HierarchyNode::leaf("DataEvent", 2313),
                            HierarchyNode::leaf("SelectionEvent", 1880),
                            HierarchyNode::leaf("TooltipEvent", 1701),
                            HierarchyNode::leaf("VisualizationEvent", 1117),
                        ],
                    ),
                    HierarchyNode::branch(
                        "legend",
                        vec![
                            HierarchyNode::leaf("Legend", 20859),
                            HierarchyNode::leaf("LegendItem", 4614),
                            HierarchyNode::leaf("LegendRange", 10530),
                        ],
                    ),
                    HierarchyNode::branch(
                        "operator",
                        vec![
                            HierarchyNode::branch(
                                "distortion",
                                vec![
                                    HierarchyNode::leaf("BifocalDistortion", 4461),
                                    HierarchyNode::leaf("Distortion", 6314),
                                    HierarchyNode::leaf("FisheyeDistortion", 3444),
                                ],
                            ),
                            HierarchyNode::branch(
                                "encoder",
                                vec![
                                    HierarchyNode::leaf("ColorEncoder", 3179),
                                    HierarchyNode::leaf("Encoder", 4060),
                                    HierarchyNode::leaf("PropertyEncoder", 4138),
                                    HierarchyNode::leaf("ShapeEncoder", 1690),
                                    HierarchyNode::leaf("SizeEncoder", 1830),
                                ],
                            ),
                            HierarchyNode::branch(
                                "filter",
                                vec![
                                    HierarchyNode::leaf("FisheyeTreeFilter", 5219),
                                    HierarchyNode::leaf("GraphDistanceFilter", 3165),
                                    HierarchyNode::leaf("VisibilityFilter", 3509),
                                ],
                            ),
                            HierarchyNode::leaf("IOperator", 1286),
                            HierarchyNode::branch(
                                "label",
                                vec![
                                    HierarchyNode::leaf("Labeler", 9956),
                                    HierarchyNode::leaf("RadialLabeler", 3899),
                                    HierarchyNode::leaf("StackedAreaLabeler", 3202),
                                ],
                            ),
                            HierarchyNode::branch(
                                "layout",
                                vec![
                                    HierarchyNode::leaf("AxisLayout", 6725),
                                    HierarchyNode::leaf("BundledEdgeRouter", 3727),
                                    HierarchyNode::leaf("CircleLayout", 9317),
                                    HierarchyNode::leaf("CirclePackingLayout", 12003),
                                    HierarchyNode::leaf("DendrogramLayout", 4853),
                                    HierarchyNode::leaf("ForceDirectedLayout", 8411),
                                    HierarchyNode::leaf("IcicleTreeLayout", 4864),
                                    HierarchyNode::leaf("IndentedTreeLayout", 3174),
                                    HierarchyNode::leaf("Layout", 7881),
                                    HierarchyNode::leaf("NodeLinkTreeLayout", 12870),
                                    HierarchyNode::leaf("PieLayout", 2728),
                                    HierarchyNode::leaf("RadialTreeLayout", 12348),
                                    HierarchyNode::leaf("RandomLayout", 870),
                                    HierarchyNode::leaf("StackedAreaLayout", 9121),
                                    HierarchyNode::leaf("TreeMapLayout", 9191),
                                ],
                            ),
                            HierarchyNode::leaf("Operator", 2490),
                            HierarchyNode::leaf("OperatorList", 5248),
                            HierarchyNode::leaf("OperatorSequence", 4190),
                            HierarchyNode::leaf("OperatorSwitch", 2581),
                            HierarchyNode::leaf("SortOperator", 2023),
                        ],
                    ),
                    HierarchyNode::leaf("Visualization", 16540),
                ],
            ),
        ],
    )
}

/// Get the top-level category names for color assignment
pub fn top_level_categories() -> Vec<&'static str> {
    vec![
        "analytics",
        "animate",
        "data",
        "display",
        "flex",
        "physics",
        "query",
        "scale",
        "util",
        "vis",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy_structure() {
        let root = flare_hierarchy();
        assert_eq!(root.name, "flare");
        assert_eq!(root.children.len(), 10); // 10 top-level categories
    }

    #[test]
    fn test_total_value() {
        let root = flare_hierarchy();
        let total = root.total_value();
        // Total should be around 1 million bytes
        assert!(total > 900_000);
        assert!(total < 1_100_000);
    }

    #[test]
    fn test_leaf_count() {
        let root = flare_hierarchy();
        let leaves = root.leaf_count();
        // Should have many leaf nodes
        assert!(leaves > 200);
    }
}
