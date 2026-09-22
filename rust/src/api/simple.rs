#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
    let _ = std::fs::create_dir_all(crate::paths::app_data_root());
    crate::process::init_job();
}
