use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write, Seek, SeekFrom};
use std::process::{Command, Output};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

use regex;
use glob::glob;
use clap::{App, Arg};

const ARCHIVE_PROGRAM_CMD: &str = "7z";

fn try_to_invert_bits(
    file: &str
) -> std::io::Result<()> {    
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(file)?;

    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer)?;
    
    for byte in buffer.iter_mut() {
        *byte = !*byte;
    }

    file.seek(SeekFrom::Start(0))?;
    file.write_all(&buffer)?;
    file.flush()?;

    Ok(())
}

fn try_to_extract_file(
    file: &str,
    password: &str,
    extracted_file: &str,
    output_directory: &str,
    invert_bits: bool
) -> std::io::Result<Output> {
    let output = if password == "" {
        Command::new(ARCHIVE_PROGRAM_CMD)
            .arg("e")        
            .arg(format!("{}", file))
            .arg(format!("{}", extracted_file))
            .arg(format!("-y"))
            .arg(format!("-o{}", output_directory))
            .output()
    } else {
        Command::new(ARCHIVE_PROGRAM_CMD)
            .arg("e")        
            .arg(format!("{}", file))
            .arg(format!("{}", extracted_file))
            .arg(format!("-p{}", password))
            .arg(format!("-y"))
            .arg(format!("-o{}", output_directory))
            .output()
    };

    //Invert all bits in buffer
    if invert_bits {
        let mut path = PathBuf::from(output_directory);
        path.push(extracted_file);
        
        if path.exists() {
            //Change permissions to writeable on windows
            #[cfg(target_os = "windows")]
            {
                let mut perms = fs::metadata(&path)?.permissions();
                perms.set_readonly(false);
                fs::set_permissions(&path, perms)?;
            }

            //Change permissions to writeable on Linux
            #[cfg(target_os = "linux")]
            {
                let mut perms = fs::metadata(&path)?.permissions();
                perms.set_mode(0o777);
                fs::set_permissions(&path, perms)?;
            }

            // When needing to use the path as a String (e.g., in `try_to_invert_bits` function):
            match try_to_invert_bits(path.to_str().unwrap()) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error inverting bits: {:?}", e);
                }
            }    
        } else {
            eprintln!("Path does not exist: {:?}", path);
        }        
    }

    output
}

fn try_to_list_files(
    file: &str,
    password: &str
) -> std::io::Result<Output> {
    if password == "" {
        return Command::new(ARCHIVE_PROGRAM_CMD)
            .arg("l")
            .arg("-r")
            .arg("-ba")
            .arg(format!("{}", file))
            .output();
    } else {
        //This is designed for 7zip version 23.01 x64 (Linux)
        //The -ba switch isn't listed in the help output, but is
        //required to suppress other verbose log messages.
        return Command::new(ARCHIVE_PROGRAM_CMD)
            .arg("l")
            .arg("-r")
            .arg("-ba")
            .arg(format!("-p{}", password))
            .arg(format!("{}", file))
            .output();
    }
}

fn try_to_tokenize_lines(output: Output) -> Vec<String> {
    let mut output_lines = Vec::new();

    // Check if the command was successful
    if output.status.success() {
        // Convert the output to a string
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Split the output into lines and tokenize each line
        for line in stdout.lines() {        
            let slice = line[53..].replace("\"","").to_string();
            output_lines.push(format!("{}", slice));
        }
    } else {
        eprintln!(
            "Command failed with error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    output_lines
}

fn main() -> std::result::Result<(), std::io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        return Ok(());
    }

    let matches = App::new("bolt")
        .about("Bolt Archive File Search")
        .arg(Arg::with_name("directory")
            .index(1)
            .required(true)
            .help("Sets the input directory"))
        .arg(Arg::with_name("output")
            .short('o')
            .required(false)
            .default_value(".")
            .help("Sets the output directory"))
        .arg(Arg::with_name("extract")
            .short('e')
            .required(false)
            .help("Extracts the files from the archive."))
        .arg(Arg::with_name("invert")
            .short('i')
            .required(false)
            .help("Inverts all bits of the output file."))
        .arg(Arg::with_name("term")
            .short('t')
            .required(false)
            .default_value(".*")
            .help("Sets the file name term to match files."))
        .arg(Arg::with_name("regex")
            .short('r')
            .required(false)
            .default_value(".*")
            .help("Sets the regular expression to match files."))
        .arg(Arg::with_name("verbose")
            .short('v')
            .required(false)
            .help("Sets the level of verbosity"))
        .arg(Arg::with_name("password")
            .short('p')
            .required(false)
            .default_value("")
            .help("Default password for files."))
        .arg(Arg::with_name("hash")
            .short('h')
            .required(false)
            .default_value("")
            .help("Default hash for files."))
        .get_matches();

    let password = matches.value_of("password").unwrap_or("");
    let directory = matches.value_of("directory").unwrap_or(".");
    let output_directory = matches.value_of("output").unwrap_or(".");
    let invert_bits = matches.is_present("invert");

    // Define the pattern to match files recursively
    let mut pattern = format!(
        "{}/**/*.7z",
        matches.value_of("directory").unwrap()
    );

    match fs::metadata(directory) {
        Ok(metadata) => {
            if metadata.is_file() {
                pattern = format!("{}", directory);
            }
        },
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    let extract = matches.is_present("extract");    
    let entries = glob(pattern.as_str()).expect("Failed to read glob pattern");

    // Use the glob function to iterate over the matching files recursively
    for entry in entries
    {
        match entry {
            Ok(path) => {
                if matches.is_present("verbose") {
                    println!("Processing archive: {}", path.display());
                }
                
                let files = try_to_tokenize_lines(
                    try_to_list_files(
                        path.to_str().unwrap(),
                        password
                    ).unwrap()
                );

                for file in files {
                    if matches.value_of("term").unwrap() != "" {
                        if !regex::Regex::new(
                            format!(".*{}.*", matches.value_of("term").unwrap()).as_str()
                        ).unwrap().is_match(format!("{}", file).as_str())
                        {
                            continue;
                        } else {
                            println!("Extracting archive: {:?}, file: {}", path.display(), file);
                            if extract {
                                let output = try_to_extract_file(
                                    path.to_str().unwrap(),
                                    password,
                                    file.replace("\"","").as_str(),
                                    output_directory,
                                    invert_bits
                                ).unwrap();
                                if matches.is_present("verbose") {
                                    println!("Output: {:?}", output);
                                }
                            }
                        }                        
                    } else {
                        if regex::Regex::new(
                                matches.value_of("regex").unwrap()
                            ).unwrap().is_match(format!("{}", file).as_str())
                        {
                            println!("Extracting archive: {:?}, file: {}", path.display(), file);
                            if extract {
                                let output = try_to_extract_file(
                                    path.to_str().unwrap(),
                                    password,
                                    file.replace("\"","").as_str(),
                                    output_directory,
                                    invert_bits
                                ).unwrap();
                                if matches.is_present("verbose") {
                                    println!("Output: {:?}", output);
                                }
                            }                        
                        }
                    }
                }
            },
            Err(e) => println!("Error: {:?}", e),
        }
    }

    Ok(())
}