# Bolt
Bolt searches for 7-zip archive files and extracts only the filenames that match either a hash or regular expression.

This tool is used to aid in the development of Linux security products focused on analyzing artifacts from vx-underground.org and VirusSign.

# Usage
Build with Cargo:
```bash
cargo build
```

Run with Cargo:
```bash
cargo run -- -h
Usage: bolt [OPTIONS] <DIRECTORY>

Arguments:
  <DIRECTORY>  [default: .]

Options:
  -m, --manifest             Sets the manifest file to generate.
  -a, --all                  Extracts all files from the archive.
  -i, --invert               Inverts all bits of the output file.
  -v, --verbose              Increase verbosity.
  -e, --extract              Extracts the files from the archive.
  -o, --output <OUTPUT>      Sets the output directory. [default: .]
  -r, --regex <REGEX>        Sets the regular expression to match files. [default: .*]
  -t, --term <TERM>          Sets the file name term to match files.
  -p, --password <PASSWORD>  Use archive password. [default: ]
  -h, --help                 Print help

cargo run -- -t hash -pinfected --extract --verbose "/mnt/drive/somefolder"
```
