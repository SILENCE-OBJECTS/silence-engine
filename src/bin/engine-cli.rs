use std::io::{self, Read, Write};
use silence_engine::{EngineInput, compute_schedule, validate_output, verify_determinism};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 { eprintln!("Usage: engine-cli <compute|verify>"); std::process::exit(1); }
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let input: EngineInput = serde_json::from_str(&s).unwrap();
    match args[1].as_str() {
        "compute" => {
            let out = compute_schedule(&input);
            validate_output(&input, &out).unwrap();
            io::stdout().write_all(serde_json::to_string_pretty(&out).unwrap().as_bytes()).unwrap();
        }
        "verify" => {
            if verify_determinism(&[input]) { println!("DETERMINISM_OK"); } else { println!("DETERMINISM_FAIL"); std::process::exit(1); }
        }
        _ => std::process::exit(1),
    }
}
