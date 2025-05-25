fn main() {
    let config = rusty_markets::Config::default();
    let json = serde_json::to_string_pretty(&config).unwrap();
    println!("{json}");
}
