// derive: ComponentBuilder
#[derive(ComponentBuilder)]
pub struct SnapshotBuilder {
    /// Element id.
    #[field(required, into)]
    pub id: String,
    /// Optional label.
    #[field(optional, into)]
    pub label: Option<String>,
    #[field(default = "true")]
    pub enabled: bool,
}
