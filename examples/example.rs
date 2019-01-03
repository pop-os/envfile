extern crate envfile;

use envfile::EnvFile;
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    let mut envfile = EnvFile::new(Path::new("examples/test.env"))?;

    for (key, value) in &envfile.store {
        println!("{}: {}", key, value);
    }

    envfile.update("ID", "example");
    println!("ID: {}", envfile.get("ID").unwrap_or(""));

    // envfile.write()?;

    Ok(())
}
