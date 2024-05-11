use std::fs::{read_to_string, File};
use std::io::{Read, Write};

mod ast;
mod compiler;
mod op_codes;
mod parser;
mod runtime;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let flag = args[1].clone();
    let path = std::path::Path::new(args[2].as_str());

    match flag.as_str() {
        "--compile" => {
            // Parse the "add.zod" file with the binary text representation.
            let zod = read_to_string(path).expect("Failed to read zod file.");
            let ast = parser::parse(&zod);

            // Compile the binary text representation to binary binary code and save the
            // compiled module in the file "add.binary"
            let binary = compiler::compile(&ast);
            let file_name = format!(
                "{}.bin",
                path.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .split(".")
                    .collect::<Vec<&str>>()[0]
            );
            let mut file = File::create(&file_name).expect("Failed to create binary file.");
            file.write_all(&binary)
                .expect("Failed to write binary file.");
            println!(">> {}", file_name);
        }
        "--execute" => {
            // Determine the function name to run and its arguments
            let func = args[3].clone();
            let func_args = &args[4..]
                .into_iter()
                .map(|i| i.parse().unwrap())
                .collect::<Vec<i32>>();

            // Read the compiled binary module "add.binary" and execute the function "add" from it.
            let mut binary = vec![];
            File::open(path).unwrap().read_to_end(&mut binary).unwrap();
            let result = runtime::invoke_function(binary, &func, func_args).unwrap();

            println!(">> {}", result);
        }
        _ => {}
    }
}
