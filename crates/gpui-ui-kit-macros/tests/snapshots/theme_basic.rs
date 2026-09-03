// derive: ComponentTheme
#[derive(ComponentTheme)]
pub struct SnapshotTheme {
    #[theme(default = 0x007acc, from = accent)]
    pub primary: u32,
    #[theme(default_f32 = 0.5, from_expr = "0.5")]
    pub opacity: f32,
}
