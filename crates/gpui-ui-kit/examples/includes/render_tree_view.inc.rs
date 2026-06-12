impl Showcase {
    fn render_tree_view_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTreeView);
        let theme = cx.theme();

        let nodes = vec![
            TreeNode::new("src", "src/")
                .icon("/")
                .children(vec![
                    TreeNode::new("main", "main.rs").icon("*").leaf(true),
                    TreeNode::new("lib", "lib.rs").icon("*").leaf(true),
                    TreeNode::new("utils", "utils/")
                        .icon("/")
                        .children(vec![
                            TreeNode::new("helpers", "helpers.rs").icon("*").leaf(true),
                        ]),
                ]),
            TreeNode::new("tests", "tests/")
                .icon("/")
                .children(vec![
                    TreeNode::new("test1", "test_main.rs").icon("*").leaf(true),
                    TreeNode::new("test2", "test_lib.rs").icon("*").leaf(true),
                ]),
            TreeNode::new("cargo-toml", "Cargo.toml").icon("*").leaf(true),
        ];

        let mut expanded_set = HashSet::new();
        expanded_set.insert(SharedString::from("src"));
        expanded_set.insert(SharedString::from("tests"));

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                div()
                    .w_full()
                    .max_w(px(400.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_2()
                    .child(
                        TreeView::new("file-tree", nodes)
                            .expanded(expanded_set)
                            .selected("main"),
                    ),
            )
    }
}
