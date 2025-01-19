// https://doc.rust-lang.org/cargo/reference/build-scripts.html
// https://doc.rust-lang.org/cargo/reference/build-script-examples.html#linking-to-system-libraries
// https://docs.rust-embedded.org/book/interoperability/c-with-rust.html

use anyhow::Result;
use rust_tcp_vpn::parsing;
use rust_tcp_vpn::run;

fn main() -> Result<()> {
    let args = parsing::parse_arg()?;
    run(args)?;
    Ok(())
}
