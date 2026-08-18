//Integration tests for ../deploy_to_github_io.sh - specifically the sed rewrite that turns
//trunk's absolute asset paths ("/www-<hash>.js", "/index-<hash>.css", ...) into paths relative
//to docs/index.html. GitHub Pages serves this site from a repo subpath
//(https://user.github.io/arbor/), not the domain root, so an absolute "/www-..." path 404s there
//even though it works fine locally where trunk serves from the root. This is a real regression
//that shipped once already: an earlier version of the rewrite only handled "/index" paths, which
//happened to fix the CSS link but left the JS/wasm bundle (named after the crate, "www-*")
//pointing at the domain root.
//
//These tests run the actual sed expression from the live script (kept in sync via
//`the_live_script_still_uses_this_exact_rewrite`, which fails loudly if the script changes
//without this file being updated to match) against realistic trunk output, rather than
//reimplementing the rewrite logic in Rust and testing that instead.
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command,Stdio};

const REWRITE_EXPR: &str = "s#/index#./index#g; s#/www#./www#g";

fn deploy_script() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../deploy_to_github_io.sh");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("couldn't read {}: {}",path.display(),e))
}

fn run_rewrite(input: &str) -> String {
    let mut child = Command::new("sed")
        .arg("-e").arg(REWRITE_EXPR)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("sed must be available on PATH to run this test");

    child.stdin.take().unwrap().write_all(input.as_bytes()).expect("failed to write to sed's stdin");
    let output = child.wait_with_output().expect("sed did not run to completion");
    assert!(output.status.success(),"sed exited with a failure status");
    String::from_utf8(output.stdout).expect("sed produced non-UTF8 output")
}

#[test]
fn the_live_script_still_uses_this_exact_rewrite() {
    let script = deploy_script();
    assert!(script.contains(REWRITE_EXPR),
        "deploy_to_github_io.sh no longer contains the expected sed expression ({:?}) - \
         if the rewrite was intentionally changed, update REWRITE_EXPR in this test to match \
         and re-verify the other tests in this file still hold for the new expression",
        REWRITE_EXPR);
}

//Regression test for the actual bug fixed in this repo: trunk names the JS/wasm bundle after the
//crate ("www-<hash>.*"), not "index-<hash>.*", so a rewrite that only handles "/index" leaves the
//wasm/js asset references absolute, which 404 under GitHub Pages' subpath hosting.
#[test]
fn rewrites_both_the_css_and_the_wasm_bundle_asset_paths() {
    let trunk_output = concat!(
        r#"<script type=module>import a,*as b from"/www-8d797501599b0478.js";"#,
        r#"a(`/www-8d797501599b0478_bg.wasm`);window.wasmBindings=b</script>"#,
        r#"<link href=/index-992a068a28534a5.css rel=stylesheet>"#,
    );

    let rewritten = run_rewrite(trunk_output);

    assert!(rewritten.contains(r#"from"./www-8d797501599b0478.js""#),
        "the JS module import should become relative:\n{}",rewritten);
    assert!(rewritten.contains("`./www-8d797501599b0478_bg.wasm`"),
        "the wasm URL should become relative:\n{}",rewritten);
    assert!(rewritten.contains("href=./index-992a068a28534a5.css"),
        "the CSS href should become relative:\n{}",rewritten);
    assert!(!rewritten.contains("href=/index"),
        "no absolute /index path should remain:\n{}",rewritten);
    assert!(!rewritten.contains(r#"from"/www"#),
        "no absolute /www path should remain:\n{}",rewritten);
}

#[test]
fn leaves_unrelated_absolute_paths_untouched() {
    let input = r#"<link rel="icon" href="/favicon.ico">"#;
    let rewritten = run_rewrite(input);
    assert_eq!(rewritten.trim_end(),input,
        "a path with neither /index nor /www should be left alone");
}

#[test]
fn the_checked_in_docs_site_has_no_leftover_absolute_asset_paths() {
    //docs/ is the actual file GitHub Pages serves. If a future deploy ever skips or breaks the
    //rewrite step, this catches it directly in the committed output rather than only in the
    //script's logic.
    let docs_index = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/index.html");
    let html = match fs::read_to_string(&docs_index) {
        Ok(html) => html,
        Err(_) => return, // docs/index.html is a build artifact; skip if it hasn't been generated yet
    };

    assert!(!html.contains("href=/index"),"docs/index.html has a leftover absolute /index path:\n{}",html);
    assert!(!html.contains(r#"from"/www"#),"docs/index.html has a leftover absolute /www path:\n{}",html);
}
