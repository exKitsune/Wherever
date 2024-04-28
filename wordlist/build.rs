use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

fn write_array<W: Write>(mut out: W, list: &str, name: &str) -> io::Result<()> {
    write!(&mut out, "pub const {}: &[&str] = &[", name)?;
    for line in list
        .lines()
        .flat_map(|l| l.split_whitespace().skip(1).next())
    {
        write!(&mut out, "\"{}\", ", line)?;
    }
    writeln!(&mut out, "];")?;
    Ok(())
}

fn main() -> io::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut file = File::create(Path::new(&out_dir).join("words.rs")).unwrap();
    #[cfg(feature = "eff_large")]
    write_array(
        &mut file,
        include_str!("eff_large_wordlist.txt"),
        "EFF_LARGE",
    )?;
    #[cfg(feature = "eff_short_1")]
    write_array(
        &mut file,
        include_str!("./eff_short_wordlist_1.txt"),
        "EFF_SHORT_1",
    )?;
    #[cfg(feature = "eff_short_2")]
    write_array(
        &mut file,
        include_str!("./eff_short_wordlist_2_0.txt"),
        "EFF_SHORT_2",
    )?;
    Ok(())
}
