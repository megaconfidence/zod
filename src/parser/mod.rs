use crate::ast::Module;

mod ctx;
mod instr;
mod module;
mod token;
mod types;
mod values;

pub fn parse(zod: &str) -> Module {
    let (_, ast) = module::module(zod).expect("Ups, something went wrong!");
    ast
}
