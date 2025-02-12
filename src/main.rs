use clap::{Parser, Subcommand};
use monistode_assemblers::stack;
use monistode_binutils::{Executable, ObjectFile, Serializable};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Assemble source files into object files
    As {
        /// Input assembly file
        input: PathBuf,

        /// Output object file
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Assembly target type
        #[arg(short = 't', long, default_value = "stack")]
        target: String,
    },

    /// Link object files into an executable
    Link {
        /// Input object files
        input: Vec<PathBuf>,

        /// Output executable file
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

fn assemble_file(input_path: &PathBuf, target: &str) -> Result<ObjectFile, String> {
    // Read input file
    let input =
        fs::read_to_string(input_path).map_err(|e| format!("Failed to read input file: {}", e))?;

    // Parse based on target
    match target {
        "stack" => stack::parse(&input).map_err(|e| format!("{}", e)),
        _ => Err(format!("Unsupported target type: {}", target)),
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::As {
            input,
            output,
            target,
        } => {
            // Determine output path
            let output_path = output.clone().unwrap_or_else(|| {
                let mut path = input.clone();
                path.set_extension("o");
                path
            });

            // Assemble the file
            match assemble_file(input, target) {
                Ok(object_file) => {
                    // Serialize the object file
                    let serialized = object_file.serialize();

                    // Write to output file
                    match fs::write(&output_path, &serialized) {
                        Ok(_) => println!("Successfully wrote object file to {:?}", output_path),
                        Err(e) => eprintln!("Failed to write output file: {}", e),
                    }
                }
                Err(e) => eprintln!("{}", e),
            }
        }

        Commands::Link { input, output } => {
            // Determine output path
            let output_path = output.clone().unwrap_or_else(|| {
                let mut path = input[0].clone();
                path.set_extension("x");
                path
            });

            // Read and merge all input object files
            let mut merged_object: Option<ObjectFile> = None;

            for path in input {
                match fs::read(path) {
                    Ok(bytes) => match ObjectFile::deserialize(&bytes) {
                        Ok((_, object_file)) => match merged_object.take() {
                            Some(mut existing) => {
                                existing.merge(object_file);
                                merged_object = Some(existing);
                            }
                            None => merged_object = Some(object_file),
                        },
                        Err(e) => {
                            eprintln!("Failed to deserialize object file {:?}: {:?}", path, e);
                            return;
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to read object file {:?}: {}", path, e);
                        return;
                    }
                }
            }

            // Link the merged object file
            if let Some(object_file) = merged_object {
                match Executable::try_from(object_file) {
                    Ok(executable) => {
                        let serialized = executable.serialize();
                        match fs::write(&output_path, &serialized) {
                            Ok(_) => println!("Successfully wrote executable to {:?}", output_path),
                            Err(e) => eprintln!("Failed to write executable: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Linking failed: {:?}", e),
                }
            } else {
                eprintln!("No input files provided");
            }
        }
    }
}
