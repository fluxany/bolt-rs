#![allow(unused_assignments)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::env;
use std::io::{Error, Result};
use std::fs::File;
use std::io::{self, Read};
use std::process::{Command, Output};
use std::path::Path;

use regex;
use glob::glob;
use clap::{App, Arg};

const ARCHIVE_PROGRAM_CMD: &str = "7z";

fn try_to_extract_file(file: &str, password: &str) -> std::io::Result<Output> {
    //This is designed for 7zip version 23.01 x64 (Linux)
    //The -ba switch isn't listed in the help output, but is
    //required to suppress other verbose log messages.
    Command::new(ARCHIVE_PROGRAM_CMD)
        .arg("l")
        .arg("-r")
        .arg("-ba")
        .arg(format!("-p{}", password))
        .arg(format!("{}", file))
        .output()
}

fn try_to_tokenize_lines(output: Output) -> Vec<String> {
    let mut output_lines = Vec::new();

    // Check if the command was successful
    if output.status.success() {
        // Convert the output to a string
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Split the output into lines and tokenize each line
        for line in stdout.lines() {        
            // Split the line into tokens (whitespace delimited)
            let tokens: Vec<&str> = line.split_whitespace().collect();            
            let slice = tokens.last().unwrap();
            output_lines.push(format!("{:?}", slice));
        }
    } else {
        eprintln!(
            "Command failed with error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    output_lines
}

#[tokio::main]
async fn main() -> std::result::Result<(), std::io::Error> {
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

    // Define the pattern to match files recursively
    let pattern = format!(
        "{}/**/*.7z",
        matches.value_of("directory").unwrap()
    );

    // Use the glob function to iterate over the matching files recursively
    for entry in glob(pattern.as_str())
        .expect("Failed to read glob pattern") 
    {
        match entry {
            Ok(path) => {
                let files = try_to_tokenize_lines(
                    try_to_extract_file(
                        path.to_str().unwrap(),
                        password
                    ).unwrap()
                );

                for file in files {
                    if matches.value_of("hash").unwrap() != "" {
                        if !regex::Regex::new(
                            format!(".*{}.*", matches.value_of("hash").unwrap()).as_str()
                        ).unwrap().is_match(format!("{}", file).as_str())
                        {                            
                            continue;
                        }
                        println!("Archive: {:?}, File: {}", path.display(), file);
                    } else {
                        if regex::Regex::new(
                                matches.value_of("regex").unwrap()
                            ).unwrap().is_match(format!("{}", file).as_str())
                        {
                            println!("Archive: {:?}, File: {}", path.display(), file);
                        }
                    }
                }
            },
            Err(e) => println!("Error: {:?}", e),
        }
    }

    Ok(())
}