// https://doc.rust-lang.org/cargo/reference/build-scripts.html
// https://doc.rust-lang.org/cargo/reference/build-script-examples.html#linking-to-system-libraries
// https://docs.rust-embedded.org/book/interoperability/c-with-rust.html

use rust_tcp_vpn::parsing;
use rust_tcp_vpn::run;

fn main() -> std::io::Result<()> {
    let args = parsing::parse_arg();
    if let Err(err) = run(args) {
        eprintln!("Execution failed for: {}", err);
        std::process::exit(1);
    };

    Ok(())
}
