use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../web/package.json");
    println!("cargo:rerun-if-changed=../web/bun.lock");
    println!("cargo:rerun-if-changed=../web/svelte.config.js");
    println!("cargo:rerun-if-changed=../web/vite.config.js");
    println!("cargo:rerun-if-changed=../web/src");

    if env::var_os("SKIP_WEB_BUILD").is_some() {
        return;
    }

    let web_dir = Path::new("../web");
    if !web_dir.join("package.json").exists() {
        return;
    }

    run_bun(web_dir, &["install", "--frozen-lockfile"], true);
    run_bun(web_dir, &["run", "build"], false);
}

fn run_bun(web_dir: &Path, args: &[&str], allow_fallback: bool) {
    let status = Command::new("bun")
        .args(args)
        .current_dir(web_dir)
        .status()
        .unwrap_or_else(|err| panic!("failed to run bun {}: {err}", args.join(" ")));

    if status.success() {
        return;
    }

    if allow_fallback && args == ["install", "--frozen-lockfile"] {
        let fallback = Command::new("bun")
            .arg("install")
            .current_dir(web_dir)
            .status()
            .unwrap_or_else(|err| panic!("failed to run bun install: {err}"));

        if fallback.success() {
            return;
        }
    }

    panic!("bun {} failed", args.join(" "));
}
