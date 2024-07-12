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
cargo run -- --help
bolt 
Bolt Archive File Search

USAGE:
    bolt.exe [OPTIONS] <directory>

ARGS:
    <directory>    Sets the input directory

OPTIONS:
    -e                   Extracts the files from the archive.
    -h <hash>            Default hash for files. [default: ]
        --help           Print help information
    -i                   Inverts all bits of the output file.
    -o <output>          Sets the output directory [default: .]
    -p <password>        Default password for files. [default: ]
    -r <regex>           Sets the regular expression to match files. [default: .*]
    -t <term>            Sets the file name term to match files. [default: .*]
    -v                   Sets the level of verbosity

cargo run -- -h hash -pinfected --extract --verbose "/mnt/drive/somefolder"
```
