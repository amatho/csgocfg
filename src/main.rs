fn main() {
    if let Err(e) = csgocfg::run() {
        eprintln!("Error: {}", e);
    }
}
