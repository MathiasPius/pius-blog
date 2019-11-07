lazy_static! {
    static ref SYNTAXSET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEMESET: ThemeSet = ThemeSet::load_defaults();
}

const DEFAULT_THEME: &str = "base16-ocean.dark";

fn accentuate(color: Color, degree: u8) -> String {
    format!("rgba({}, {}, {}, {})",
        max(color.r, degree) - degree,
        max(color.g, degree) - degree,
        max(color.b, degree) - degree,
        color.a
    )
}