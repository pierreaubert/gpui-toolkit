// derive: ComponentVariant
// expect-error: only supports unit variants
#[derive(ComponentVariant)]
pub enum VariantWithFields {
    Unit,
    Tuple(u8),
}
