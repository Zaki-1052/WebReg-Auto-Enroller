use std::fs;
use toml::Value;

fn main() {
    let contents = fs::read_to_string("config.toml").expect("Failed to read file");
    let value: Value = toml::from_str(&contents).expect("Failed to parse TOML");
    println!("Successfully parsed TOML: {:#?}", value);
}
