// derive: ComponentTheme
// expect-error: Unknown theme attribute
#[derive(ComponentTheme)]
pub struct UnknownKeyTheme {
    #[theme(default = 0x007acc, from = accent, bogus = 1)]
    pub primary: u32,
}
