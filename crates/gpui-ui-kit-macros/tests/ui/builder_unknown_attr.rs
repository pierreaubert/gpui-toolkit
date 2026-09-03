// derive: ComponentBuilder
// expect-error: unknown builder field attribute
#[derive(ComponentBuilder)]
pub struct UnknownAttrBuilder {
    #[field(frobnicate)]
    pub id: String,
}
