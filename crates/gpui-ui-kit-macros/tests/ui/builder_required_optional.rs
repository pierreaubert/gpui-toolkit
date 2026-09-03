// derive: ComponentBuilder
// expect-error: cannot be both required and optional
#[derive(ComponentBuilder)]
pub struct RequiredOptionalBuilder {
    #[field(required, optional)]
    pub id: String,
}
