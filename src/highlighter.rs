use std::cmp::max;
use std::path::Path;
use std::io::BufRead;
use std::collections::HashMap;
use tera::{Result, Value};
use syntect::{
    parsing::SyntaxSet,
    html::{
        IncludeBackground,
        start_highlighted_html_snippet,
        append_highlighted_html_for_styled_line
    },
    highlighting::{Color, Theme, ThemeSet},
    easy::HighlightFile
};

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

// This function is lifted more or less one-to-one from syntect, but adds the <code></code>
// which allows us to do nice line numbering on em
fn syntax_highlighter<P: AsRef<Path>>(path: P, theme: &Theme) -> std::io::Result<String> {
    let mut highlighter = HighlightFile::new(path, &SYNTAXSET, theme)?;
    let (mut output, bg) = start_highlighted_html_snippet(theme);

    let numbering = theme.settings.gutter_foreground
        .or(theme.settings.foreground)
        .unwrap_or(Color::BLACK);

    output.push_str(&format!("<style type=\"text/css\">pre code::before {{ color: rgba({}, {}, {}, {}); }}</style>",
        numbering.r, numbering.g, numbering.b, numbering.a
    ));

    // This is an alternate background color used for every second line to distinguish them
    let extras = theme.settings.background.map(|c|
        format!("style=\"background-color: {};\"", accentuate(c, 5))
    ).unwrap_or("".into());

    let mut line = String::new();
    let mut alternate = false;
    while highlighter.reader.read_line(&mut line)? > 0 {
        {
            if alternate {
                output.push_str(&format!("<code {}>", extras));
            } else {
                output.push_str("<code>");
            }
            alternate = !alternate;

            let regions = highlighter.highlight_lines.highlight(&line, &SYNTAXSET);
            append_highlighted_html_for_styled_line(&regions[..], IncludeBackground::IfDifferent(bg), &mut output);
            output.push_str("</code>");
        }
        line.clear();
    }
    output.push_str("</pre>\n");
    Ok(output)
}


pub fn highlight(args: HashMap<String, Value>) -> Result<Value>{
    // Extracting the chosen theme is split into two blocks like this
    // to prevent heap-allocating the default theme string each time
    let chosen_theme = match args.get("theme") {
        Some(value) => tera::from_value::<String>(value.clone())
            .map(|x| Some(x))
            .unwrap_or(None),
        _ => None
    };

    let theme = match &chosen_theme {
        Some(theme) => &THEMESET.themes[theme],
        None => &THEMESET.themes[DEFAULT_THEME]
    };


    if let Some(value) = args.get("file") {
        if let Ok(filename) = tera::from_value::<String>(value.clone()) { 
            let html = syntax_highlighter(&filename, &theme)
                .map_err(|e| tera::Error::from(
                    format!("failed to generate syntax highlighting for {}: {}", &filename, e)
                ))?;

            return Ok(tera::to_value(html)?);
        }
    }

    Err(tera::Error::from(format!("missing file or text parameter")))
}

pub fn codeblock(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    if let Ok(source) = tera::from_value::<String>(value) {
        return Ok(tera::to_value(&format!("<div class=\"codeblock\">{}</div>", source))
            .map_err(|e| tera::Error::from(format!("failed to serialize codeblock: {}", e)))?
        );
    }

    Err(tera::Error::from(format!("missing input to codeblock function")))
}