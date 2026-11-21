//! Links the extension module the way Python expects on each platform.

fn main() {
    pyo3_build_config::add_extension_module_link_args();
}
