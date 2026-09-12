pub mod ast;
pub mod borrowck;
pub mod lexer;
pub mod parser;
pub mod backend; // [NUEVO]

use logos::Logos;
use lexer::Token;
use parser::Parser;
use borrowck::{BorrowContext, check_statement};
use backend::generate_x86_64_bare_metal;

fn main() {
    println!("=== Pipeline de Transducción Completo: Sovereign OS Compiler ===\n");

    let source_code = r#"
    {
        let buffer: owned u8 = 1024;
        let number: u32 = 42;
        
        unsafe {
            let mmio_port: raw u32 = 400000;
        }
    }
    "#;

    let lexer = Token::lexer(source_code);
    let tokens: Vec<Token> = lexer.filter_map(Result::ok).collect();

    let mut parser = Parser::new(tokens.into_iter());
    match parser.parse_block() {
        Ok(ast) => {
            let mut ctx = BorrowContext::new();
            match check_statement(&ast, &mut ctx) {
                Ok(transformed_ast) => {
                    // FASE FINAL: Emisión de código
                    println!("\n[3] Backend: Generando Código Máquina (x86_64 Bare-Metal)...");
                    let mut final_asm = String::new();
                    for node in transformed_ast {
                        final_asm.push_str(&generate_x86_64_bare_metal(&node));
                    }
                    
                    println!("\n--- OUTPUT ENSAMBLADOR (POC COMPLETADO) ---");
                    println!("{}", final_asm);
                    println!("------------------------------------------");
                }
                Err(e) => eprintln!("\n[!] FALLO TERMODINÁMICO DETECTADO: {}", e),
            }
        }
        Err(e) => eprintln!("\n[!] ERROR SINTÁCTICO: {}", e),
    }
}
