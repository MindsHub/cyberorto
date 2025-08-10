fn main() {
    #[cfg(feature = "defmt")]
    println!("cargo::rustc-link-arg=-Tdefmt.x");

    #[cfg(test)]
    println!("cargo::rustc-link-arg-tests=-Tembedded-test.x");
    println!("cargo::rustc-link-arg=-Tlink.x");
}
