pub mod ast;
pub mod borrowck;
pub mod lexer;
pub mod parser;
pub mod ir;
pub mod lowering;
pub mod backend;

use logos::Logos;
use lexer::Token;
use parser::Parser;
use borrowck::{BorrowContext, check_statement};
use lowering::IrLoweringVisitor;
use backend::generate_x86_64_from_ir;

fn main() {
    println!("=================================================================");
    println!("   SOVEREIGN COMPILER v0.2.0 - C5-REAL ARCHITECTURE             ");
    println!("   BIFURCACIÓN 2: DESCENSO AL SILICIO (SSA IR & CFG LOWERING)   ");
    println!("=================================================================\n");

    // Código fuente demostrando los tres imperativos:
    // 1. Owned buffer (1024 bytes) -> Materialización de SyntheticDrop
    // 2. Escalar Copy (number: 42) -> Libre de destrucción
    // 3. Puntero MMIO en Ring-0 (*mmio_port = number) -> VolatileStore inmutable
    let source_code = r#"
    {
        let buffer: owned u8 = 1024;
        let number: u32 = 42;
        
        unsafe {
            let mmio_port: raw u32 = 400000;
            *mmio_port = number;
        }
    }
    "#;

    println!("[1] Código Fuente Original (Territorio):");
    println!("{}\n", source_code.trim());

    // --- FASE 1: Lexer ---
    let lexer = Token::lexer(source_code);
    let tokens: Vec<Token> = lexer.filter_map(Result::ok).collect();
    println!("[2] Lexer: Compresión a Tokens completada ({} tokens generados).", tokens.len());

    // --- FASE 2: Parser ---
    let mut parser = Parser::new(tokens.into_iter());
    let ast = match parser.parse_block() {
        Ok(tree) => {
            println!("[3] Parser: Árbol Sintáctico Abstracto (AST) proyectado con éxito.");
            tree
        }
        Err(e) => {
            eprintln!("\n[!] ERROR SINTÁCTICO: {}", e);
            return;
        }
    };

    // --- FASE 3: Borrow Checker ---
    println!("\n[4] Borrow Checker: Verificando Semántica Afín e Inyectando Drops...");
    let mut ctx = BorrowContext::new();
    let transformed_ast = match check_statement(&ast, &mut ctx) {
        Ok(nodes) => {
            println!("[+] ÉXITO: Invariante C5-REAL Mantenida (Markov Blanket estable).");
            nodes
        }
        Err(e) => {
            eprintln!("\n[!] FALLO TERMODINÁMICO: {}", e);
            return;
        }
    };

    // --- FASE 4: Lowering a SSA IR (CFG) ---
    println!("\n[5] Lowering Engine: Aplanamiento Topológico hacia SSA IR...");
    let visitor = IrLoweringVisitor::new();
    let function_ir = visitor.lower_ast(&transformed_ast, "kernel_main");

    println!("\n=== GRAFO DE FLUJO DE CONTROL (CFG SSA IR) ===");
    println!("{}", function_ir);
    println!("==============================================\n");

    // --- FASE 5: Backend de Código Máquina ---
    println!("[6] Backend: Emisión de Código Máquina (x86_64 Bare-Metal Ring-0)...");
    let asm_output = generate_x86_64_from_ir(&function_ir);

    println!("\n--- OUTPUT ENSAMBLADOR EMITIDO ---");
    println!("{}", asm_output);
    println!("----------------------------------");
}
