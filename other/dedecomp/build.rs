fn main() {
    println!("cargo::rerun-if-changed=src/rnc.c");
    cc::Build::new().file("src/rnc.c").compile("rnc");
}