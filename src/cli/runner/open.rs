use std::path::Path;

use super::dispatch::print_error;
use crate::config;

pub fn run_open(
    config_override: Option<&Path>,
    target_arg: Option<&str>,
    print_url: bool,
    browser_override: Option<&str>,
) -> i32 {
    let (root_dir, tag_prefix) = match config_override {
        Some(path) => match config::load(path) {
            Ok(c) => (c.root_dir, c.git.tag_prefix),
            Err(e) => {
                print_error(e);
                return 1;
            }
        },
        None => match std::env::current_dir() {
            Ok(d) => match config::discover(&d) {
                Ok(c) => (c.root_dir, c.git.tag_prefix),
                Err(_) => (d, "v".to_string()),
            },
            Err(e) => {
                print_error(format!("failed to determine current directory: {e}"));
                return 1;
            }
        },
    };

    let target = crate::cli::open::OpenTarget::parse(target_arg);
    let url = match crate::cli::open::resolve_url_for_repo(&root_dir, &target, &tag_prefix) {
        Ok(u) => u,
        Err(e) => {
            print_error(e);
            return 1;
        }
    };

    if print_url {
        println!("{url}");
        return 0;
    }

    match crate::cli::open::launch_browser(&url, browser_override) {
        Ok(()) => 0,
        Err(e) => {
            print_error(e);
            1
        }
    }
}
