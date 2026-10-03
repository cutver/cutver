/// Normalizes command-line arguments to handle custom short alias flags
/// before handing them off to `clap`.
pub fn normalize_args<I, T>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let mut normalized = Vec::new();
    let mut in_init = false;
    for arg in args {
        let s = arg.into();
        if s == "init" {
            in_init = true;
            normalized.push(s);
        } else if s == "-fr" {
            normalized.push("--first-release".to_string());
        } else if in_init && s == "-nt" {
            normalized.push("--no-template".to_string());
        } else {
            normalized.push(s);
        }
    }
    normalized
}
