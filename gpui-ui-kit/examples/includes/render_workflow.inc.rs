use gpui_ui_kit::workflow::{Position, WorkflowNodeData};

impl Showcase {
    fn render_workflow_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();
        
        // Get stats from canvas
        let (node_count, connection_count, _selected_count) = self.workflow_canvas.read(cx).stats();

        div()
            .flex()
            .flex_col()
            .gap_6()
            .h_full() // Use full height for canvas
            .child(self.section_header("Workflow Canvas"))
            .child(
                Text::new("A node-based workflow editor with drag-and-drop connections, panning, and zooming.")
                    .color(theme.text_secondary)
            )
            
            // Canvas Container
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_xl()
                    .overflow_hidden()
                    .h(px(500.0)) // Fixed height for the demo area
                    
                    // Toolbar
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_3()
                            .border_b_1()
                            .border_color(theme.border)
                            .bg(theme.surface)
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(
                                        Button::new("wf-add-node", "Add Node")
                                            .size(ButtonSize::Sm)
                                            .on_click({
                                                let entity = entity.clone();
                                                move |_, cx| {
                                                    entity.update(cx, |this, cx| this.workflow_add_node(cx));
                                                }
                                            }),
                                    )
                                    .child(
                                        Button::new("wf-clear", "Clear")
                                            .size(ButtonSize::Sm)
                                            .variant(ButtonVariant::Destructive)
                                            .on_click({
                                                let entity = entity.clone();
                                                move |_, cx| {
                                                    entity.update(cx, |this, cx| {
                                                        this.workflow_canvas.update(cx, |canvas, cx| canvas.clear(cx));
                                                    });
                                                }
                                            }),
                                    )
                            )
                            .child(
                                Text::new(format!("Nodes: {} | Conns: {}", node_count, connection_count))
                                    .size(TextSize::Xs)
                                    .muted(true)
                            )
                    )
                    
                    // Canvas
                    .child(
                        div()
                            .flex_1()
                            .relative()
                            .child(self.workflow_canvas.clone())
                    )
                    
                    // Footer instructions
                    .child(
                        div()
                            .p_2()
                            .bg(theme.background)
                            .border_t_1()
                            .border_color(theme.border)
                            .child(
                                Text::new("Drag nodes to move. Drag from ports to connect. Scroll to zoom. Middle-click to pan.")
                                    .size(TextSize::Xs)
                                    .muted(true)
                            )
                    )
            )
    }

    fn workflow_add_node(&mut self, cx: &mut Context<Self>) {
        self.workflow_node_counter += 1;
        let id = self.workflow_node_counter;
        
        let x = 100.0 + (id as f32 * 30.0) % 400.0;
        let y = 100.0 + (id as f32 * 20.0) % 300.0;
        
        let node = WorkflowNodeData::new(
            &format!("Node {}", id), 
            Position::new(x, y)
        )
        .with_ports(1, 1)
        .with_size(160.0, 70.0);

        self.workflow_canvas.update(cx, |canvas, cx| {
            canvas.add_node_notify(node, cx);
        });
    }
}
