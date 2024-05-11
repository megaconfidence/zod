use crate::ast::*;
use crate::op_codes::*;
use crate::runtime::error::RuntimeError;
use crate::runtime::reader::Reader;

fn check_header(binary: &Reader) -> Result<(), RuntimeError> {
    if binary.len() < 8 {
        return Err(RuntimeError::ModuleToShort);
    }

    if binary.bytes(4) != *b"\0asm" {
        return Err(RuntimeError::WrongMagicHeader);
    }

    if binary.dword() != 1 {
        return Err(RuntimeError::WrongVersionHeader);
    }

    Ok(())
}

fn parse_type_section(binary: &Reader) -> Result<Vec<Type>, RuntimeError> {
    if binary.byte() != section::TYPE {
        return Err(RuntimeError::InvalidSectionCode);
    }
    let _size = binary.byte();
    let num_types = binary.byte();
    let mut types = vec![];

    fn parse_valuetype(binary: &Reader) -> Result<ValueType, RuntimeError> {
        match binary.byte() {
            0x7f => Ok(ValueType::I32),
            0x7e => Ok(ValueType::I64),
            _ => Err(RuntimeError::InvalidValueType),
        }
    }

    for _ in 0..num_types {
        let _func = binary.byte();

        // parse params
        let mut params = vec![];
        for _ in 0..binary.byte() {
            params.push(parse_valuetype(binary)?);
        }

        // parse results
        let mut results = vec![];
        for _ in 0..binary.byte() {
            results.push(parse_valuetype(binary)?);
        }

        types.push((params, results));
    }

    Ok(types)
}

fn parse_func_section(binary: &Reader) -> Result<Vec<i32>, RuntimeError> {
    if binary.byte() != section::FUNC {
        return Err(RuntimeError::InvalidSectionCode);
    }

    let _size = binary.byte();
    let num = binary.byte();
    let mut f_types = vec![];

    for _ in 0..num {
        f_types.push(binary.byte() as i32)
    }

    Ok(f_types)
}

fn parse_export_section(binary: &Reader) -> Result<Vec<Export>, RuntimeError> {
    if binary.byte() != section::EXPORT {
        return Err(RuntimeError::InvalidSectionCode);
    }

    let _size = binary.byte();
    let num = binary.byte();
    let mut exports = vec![];

    for _ in 0..num {
        let length = binary.byte();
        let name = match std::str::from_utf8(binary.bytes(length.into())) {
            Ok(n) => n.to_string(),
            Err(_) => return Err(RuntimeError::InvalidExportName),
        };
        let _zero = binary.byte();
        let e_desc = match binary.byte() {
            0x00 => EDesc::FuncExport(0),
            _ => return Err(RuntimeError::InvalidExportType),
        };

        exports.push(Export { name, e_desc })
    }

    Ok(exports)
}

pub fn parse_code_section(binary: &Reader) -> Result<Vec<(StackType, Vec<Instr>)>, RuntimeError> {
    if binary.byte() != section::CODE {
        return Err(RuntimeError::InvalidSectionCode);
    };

    let _size = binary.byte();
    let num = binary.byte();
    let mut code = vec![];

    for _ in 0..num {
        let _size = binary.byte();
        let num_locals = binary.byte() as i32;
        let mut locals = vec![];
        let mut instrs = vec![];

        for _ in 0..num_locals {
            let vt = match binary.byte() {
                0x7f => ValueType::I32,
                0x7e => ValueType::I64,
                _ => return Err(RuntimeError::InvalidValueType),
            };
            locals.push(vt);
        }

        loop {
            let instr = match binary.byte() {
                0x20 => Instr::LocalGet(binary.byte() as usize),
                0x6a => Instr::I32Add,
                0x0b => break,
                _ => return Err(RuntimeError::InvalidInstruction),
            };

            instrs.push(instr);
        }

        code.push((locals, instrs));
    }

    Ok(code)
}

pub fn parse_binary(binary: &Reader) -> Result<Module, RuntimeError> {
    check_header(binary)?;
    let types = parse_type_section(binary)?;
    let funcs = parse_func_section(binary)?;
    let exports = parse_export_section(binary)?;
    let code = parse_code_section(binary)?;

    let join_code_func = || {
        funcs
            .iter()
            .enumerate()
            .map(|(i, f)| Func {
                f_type: *f,
                locals: code[i].0.clone(),
                body: code[i].1.clone(),
            })
            .collect::<Vec<Func>>()
    };

    Ok(Module {
        types,
        exports,
        funcs: join_code_func(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_code_section_test() {
        let binary = vec![
            0x0a, // section code
            0x09, // section size
            0x01, // num function
            // function body 0
            0x07, // func body size
            0x00, // local decl count
            0x20, // local.get
            0x00, // local index
            0x20, // local.get
            0x01, // local index
            0x6a, // i32.add
            0x0b, // end
        ];
        let reader = Reader::new(binary);

        let (locals, instructions) = parse_code_section(&reader).unwrap()[0].clone();

        assert_eq!(Vec::<ValueType>::new(), locals);
        assert_eq!(
            vec![Instr::LocalGet(0), Instr::LocalGet(1), Instr::I32Add],
            instructions
        );
    }

    #[test]
    fn parse_export_section_test() {
        let binary = vec![
            0x07, // section export
            0x07, // section size
            0x01, // num exports
            0x03, // string length
            // "add" export name
            0x61, // a
            0x64, // d
            0x64, // d
            0x00, // 0
            // export kind
            0x00, // export func index
        ];
        let reader = Reader::new(binary);

        let result = parse_export_section(&reader).unwrap();

        assert_eq!(
            vec![Export {
                name: "add".to_string(),
                e_desc: EDesc::FuncExport(0)
            }],
            result
        );
    }

    #[test]
    fn parse_func_section_test() {
        let binary = vec![
            0x03, // section code
            0x02, // section size
            0x01, // num functions
            0x00, // function 0 signature index
        ];
        let reader = Reader::new(binary);

        let result = parse_func_section(&reader).unwrap();

        assert_eq!(vec![0], result);
    }

    #[test]
    fn parse_binary_test() {
        let binary = vec![
            // binary magic
            0x00, // \0
            0x61, // a
            0x73, // s
            0x6d, // m
            // binary version
            0x01, // 1
            0x00, // 0
            0x00, // 0
            0x00, // 0
            // section "Type" (1)
            0x01, // section code
            0x07, // section size
            0x01, // num types
            // type 0
            0x60, // func
            0x02, // num params
            0x7f, // i32
            0x7f, // i32
            0x01, // num results
            0x7f, // i32
            // section "Function" (3)
            0x03, // section code
            0x02, // section size
            0x01, // num functions
            0x00, // function 0 signature index
            // section "Export" (7)
            0x07, // section export
            0x07, // section size
            0x01, // num exports
            0x03, // string length
            // "add" export name
            0x61, // a
            0x64, // d
            0x64, // d
            0x00, // 0
            // export kind
            0x00, // export func index
            // section "Code" (10)
            0x0a, // section code
            0x09, // section size
            0x01, // num function
            // function body 0
            0x07, // func body size
            0x00, // local decl count
            0x20, // local.get
            0x00, // local index
            0x20, // local.get
            0x01, // local index
            0x6a, // i32.add
            0x0b, // end
        ];
        let reader = Reader::new(binary);

        let result = parse_binary(&reader).unwrap();

        assert_eq!(
            Module {
                types: vec![(vec![ValueType::I32, ValueType::I32], vec![ValueType::I32])],
                funcs: vec![Func {
                    f_type: 0,
                    locals: vec![],
                    body: vec![Instr::LocalGet(0), Instr::LocalGet(1), Instr::I32Add],
                }],
                exports: vec![Export {
                    name: "add".to_string(),
                    e_desc: EDesc::FuncExport(0),
                }],
            },
            result
        );
    }

    #[test]
    fn check_header_test() {
        let binary = vec![
            // binary magic
            0x00, // \0
            0x61, // a
            0x73, // s
            0x6d, // m
            // binary version
            0x01, // 1
            0x00, // 0
            0x00, // 0
            0x00, // 0
        ];
        let reader = Reader::new(binary);

        assert!(check_header(&reader).is_ok());
    }

    #[test]
    fn parse_type_section_test() {
        let binary = vec![
            // section "Type" (1)
            0x01, // section code
            0x07, // section size
            0x01, // num types
            // type 0
            0x60, // func
            0x02, // num params
            0x7f, // i32
            0x7f, // i32
            0x01, // num results
            0x7f, // i32
        ];
        let reader = Reader::new(binary);

        let types = parse_type_section(&reader).unwrap();

        assert_eq!(
            types,
            vec![(vec![ValueType::I32, ValueType::I32], vec![ValueType::I32])]
        );
    }
}
