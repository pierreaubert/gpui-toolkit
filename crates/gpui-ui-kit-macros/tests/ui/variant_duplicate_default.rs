// derive: ComponentVariant
// expect-error: Multiple #[default] variants
#[derive(ComponentVariant)]
pub enum VariantDuplicateDefault {
    #[default]
    A,
    #[default]
    B,
}
