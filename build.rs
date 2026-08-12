fn main() {
    // Use pkg-config to find geany's include paths and link flags
    let geany = pkg_config::Config::new()
        .probe("geany")
        .expect("Could not find geany via pkg-config. Install libgeany-dev or geany-dev.");

    for path in &geany.include_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    println!("cargo:rustc-link-lib=dylib=geany");

    // Probe libcurl
    pkg_config::Config::new()
        .probe("libcurl")
        .expect("Could not find libcurl via pkg-config. Install libcurl4-gnutls-dev or libcurl4-openssl-dev.");

    // Strip debug symbols in release
    println!("cargo:rustc-link-arg=-s");
}
