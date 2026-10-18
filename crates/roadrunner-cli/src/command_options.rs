//! Small strict parsers for the new CLI adapters; no new runtime dependency.
use std::collections::BTreeMap;

pub(super) fn flags<'a>(
    args: &'a [String],
    allowed: &[&str],
) -> Result<(BTreeMap<&'a str, &'a str>, bool), String> {
    let mut options = BTreeMap::new();
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        let name = args[i].as_str();
        if name == "--json" {
            if json {
                return Err("duplicate --json".into());
            }
            json = true;
            i += 1;
            continue;
        }
        if !allowed.contains(&name) {
            return Err(format!("unknown option {name}; run roadrunner --help"));
        }
        let value = args
            .get(i + 1)
            .filter(|s| !s.starts_with("--"))
            .ok_or_else(|| format!("missing value for {name}"))?;
        if options.insert(name, value.as_str()).is_some() {
            return Err(format!("duplicate {name}"));
        }
        i += 2;
    }
    Ok((options, json))
}

pub(super) fn required<'a>(
    options: &BTreeMap<&str, &'a str>,
    name: &str,
) -> Result<&'a str, String> {
    options
        .get(name)
        .copied()
        .ok_or_else(|| format!("missing required {name}; run roadrunner --help"))
}
