fn main() {
    // Required for PyO3 extension modules on macOS: allows Python symbols to be
    // resolved at runtime by the Python interpreter (dynamic lookup).
    pyo3_build_config::add_extension_module_link_args();
}
