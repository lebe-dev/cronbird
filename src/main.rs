pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " #1");

fn main() {
    println!("cronbird {}", VERSION);
}
