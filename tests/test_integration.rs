mod cases;
mod common;

#[test]
fn version_sync() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}
