// derive: ComponentTheme
// expect-error: missing #[theme(...)] attribute
#[derive(ComponentTheme)]
pub struct MissingAttrTheme {
    pub primary: u32,
}
