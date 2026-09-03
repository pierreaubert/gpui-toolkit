// derive: ComponentVariant
#[derive(ComponentVariant)]
pub enum SnapshotVariant {
    #[default]
    Primary,
    Secondary,
    #[variant(name = "danger")]
    Destructive,
}
