/*******************************************************************************
 * Copyright (c) 2024 Nicholas LaRoche <nicholas.louis.laroche@outlook.com>
 *
 * This program and the accompanying materials are made available under the
 * terms of the Eclipse Public License v. 2.0 which is available at
 * http://www.eclipse.org/legal/epl-2.0.
 *
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
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
use clap::Parser;

//mod manifest;

const ARCHIVE_PROGRAM_CMD: &str = "7z";

/// Inverts all bits in a file after opening for read/write.
/// This method fails if the file cannot be opened for writing.
fn try_to_invert_bits(
    file: &str
) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(file)?;

    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer)?;
    
    // Invert all bits in the buffer.
    for byte in buffer.iter_mut() {
        *byte = !*byte;
    }

    file.seek(SeekFrom::Start(0))?;
    file.write_all(&buffer)?;
    file.flush()?;

    Ok(())
}

fn try_to_change_perms_and_invert(
    path: &PathBuf
) -> std::io::Result<()> {
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
            perms.set_mode(0o666);
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
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Path does not exist"));
    }
    Ok(())
}

/// Extracts a file from an archive using the 7z program.
/// This method returns the output of the command regardless of success.
fn try_to_extract_file(
    file: &str,
    password: &str,
    extracted_file: &str,
    output_directory: &str,
    invert_bits: bool,
    extract_all: bool
) -> std::io::Result<Output> {
    let output = if extract_all {
        if password == "" {
            Command::new(ARCHIVE_PROGRAM_CMD)
                .arg("x")        
                .arg(format!("{}", file))
                .arg(format!("-y"))
                .arg(format!("-o{}", output_directory))
                .output()
        } else {
            Command::new(ARCHIVE_PROGRAM_CMD)
                .arg("x")        
                .arg(format!("{}", file))
                .arg(format!("-p{}", password))
                .arg(format!("-y"))
                .arg(format!("-o{}", output_directory))
                .output()
        }
    } else {
        if password == "" {
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
        }    
    };

    if invert_bits {
        if extract_all {        
            let pattern = format!("{}/**", output_directory);
            let entries = glob(pattern.as_str()).expect("Failed to read glob pattern");

            // Use the glob function to iterate over the matching files recursively
            for entry in entries {
                match try_to_change_perms_and_invert(&entry.unwrap()) {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Error changing permissions and inverting bits: {:?}", e);
                    }
                }
            }
        } else {
            let mut path = PathBuf::from(output_directory);
            path.push(extracted_file);

            match try_to_change_perms_and_invert(&path) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error changing permissions and inverting bits: {:?}", e);
                }
            }
        }
    }
    output
}

/// Lists all files in an archive using the 7z program.
/// This method returns the output of the command regardless of success.
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

/// Helper function to tokenize the output of a command.
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

#[derive(Parser, Debug)]
struct Args {
    #[arg(required = true, default_value = ".")]
    directory: String,

    #[arg(short, long, help = "Sets the manifest file to generate.")]
    manifest: bool,

    #[arg(short, long, help = "Extracts all files from the archive.")]
    all: bool,

    #[arg(short, long, help = "Inverts all bits of the output file.")]
    invert: bool,

    #[arg(short, long, help = "Increase verbosity.")]
    verbose: bool,

    #[arg(short, long, help = "Extracts the files from the archive.")]
    extract: bool,

    #[arg(short, long, default_value = ".", help = "Sets the output directory.")]
    output: String,

    #[arg(short, long, default_value = ".*", help = "Sets the regular expression to match files.")]
    regex: String,

    #[arg(required = false, short, long, help = "Sets the file name term to match files.")]
    term: String,

    #[arg(short, long, default_value = "", help = "Use archive password.")]
    password: String,
}

/// Main function of the program.
/// Accepts command line options and processes the archive files as they are found.
fn main() -> std::result::Result<(), std::io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        return Ok(());
    }

    let args = Args::parse();

    let password = args.password;
    let directory = args.directory;
    let output_directory = args.output;
    let invert_bits = args.invert;
    let extract_all = args.all;

    // Define the pattern to match files recursively
    let mut pattern = format!(
        "{}/**/*.7z",
        directory
    );

    match fs::metadata(directory.clone()) {
        Ok(metadata) => {
            if metadata.is_file() {
                pattern = format!("{}", &directory);
            }
        },
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    let extract = args.extract;    
    let entries = glob(pattern.as_str()).expect("Failed to read glob pattern");

    // Use the glob function to iterate over the matching files recursively
    for entry in entries
    {
        match entry {
            Ok(path) => {
                if args.verbose {
                    println!("Processing archive: {}", path.display());
                }
                
                let files = try_to_tokenize_lines(
                    try_to_list_files(
                        path.to_str().unwrap(),
                        password.as_str()
                    ).unwrap()
                );

                for file in files {
                    if args.term != "" {
                        if !regex::Regex::new(
                            format!(".*{}.*", args.term).as_str()
                        ).unwrap().is_match(format!("{}", file).as_str())
                        {
                            continue;
                        } else {                            
                            if extract {
                                println!("Extracting archive: {:?}, file: {}", path.display(), file);
                                let output = try_to_extract_file(
                                    path.to_str().unwrap(),
                                    password.as_str(),
                                    file.replace("\"","").as_str(),
                                    output_directory.as_str(),
                                    invert_bits,
                                    extract_all
                                ).unwrap();
                                if args.verbose {
                                    println!("Output: {:?}", output);
                                }
                            }
                        }                        
                    } else {
                        if regex::Regex::new(
                                args.regex.as_str()
                            ).unwrap().is_match(format!("{}", file).as_str())
                        {                            
                            if extract {
                                println!("Extracting archive: {:?}, file: {}", path.display(), file);
                                let output = try_to_extract_file(
                                    path.to_str().unwrap(),
                                    password.as_str(),
                                    file.replace("\"","").as_str(),
                                    output_directory.as_str(),
                                    invert_bits,
                                    extract_all
                                ).unwrap();
                                if args.verbose {
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