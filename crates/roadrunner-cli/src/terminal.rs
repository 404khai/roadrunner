//! Human terminal branding, kept out of machine-readable command streams.
use std::io::IsTerminal;

pub(super) const BANNER: &str = include_str!("banner.txt");

pub(super) fn banner_if_terminal() {
    if std::io::stdout().is_terminal() {
        print!(
            "{BANNER}\nRouting & delivery optimization · v{}\n\n",
            env!("CARGO_PKG_VERSION")
        );
    }
}

pub(super) fn output(value: &impl serde::Serialize, human: &str, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
        );
    } else {
        banner_if_terminal();
        println!("{human}");
    }
    Ok(())
}
