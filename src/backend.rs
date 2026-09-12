use crate::ast::{Stmt, Expr, Type};

/// Transductor Termodinámico: AST -> Código Máquina Soberano (x86_64)
pub fn generate_x86_64_bare_metal(ast: &Stmt) -> String {
    let mut asm = String::new();
    
    // Cabecera Ring-0 (no_std)
    asm.push_str("; ==========================================\n");
    asm.push_str("; SOVEREIGN COMPILER - C5-REAL ARCHITECTURE\n");
    asm.push_str("; TARGET: x86_64 Bare-Metal (Ring-0)\n");
    asm.push_str("; ==========================================\n\n");
    asm.push_str(".global _start\n");
    asm.push_str(".section .text\n\n");
    
    asm.push_str("_start:\n");
    asm.push_str("    ; --- Prólogo del Kernel ---\n");
    asm.push_str("    mov rbp, rsp\n\n");
    
    compile_stmt(ast, &mut asm, 1);
    
    asm.push_str("\n    ; --- Kernel Panic / Halt Loop ---\n");
    asm.push_str(".halt:\n");
    asm.push_str("    cli\n");
    asm.push_str("    hlt\n");
    asm.push_str("    jmp .halt\n");
    
    asm
}

fn compile_stmt(stmt: &Stmt, asm: &mut String, depth: usize) {
    let indent = "    ".repeat(depth);
    match stmt {
        Stmt::Block(stmts) => {
            for s in stmts {
                compile_stmt(s, asm, depth);
            }
        }
        Stmt::UnsafeBlock(stmts) => {
            asm.push_str(&format!("\n{}> --- INICIO ESCOTILLA UNSAFE ---\n", indent));
            for s in stmts {
                compile_stmt(s, asm, depth);
            }
            asm.push_str(&format!("{}> --- FIN ESCOTILLA UNSAFE ---\n", indent));
        }
        Stmt::Let(name, ty, expr) => {
            let val = match expr {
                Expr::LiteralInt(v) => *v,
                _ => 0, // Mock para el POC
            };
            
            if let Type::RawPtr(_) = ty {
                asm.push_str(&format!("{}mov rax, {:#x} ; [MMIO] {} = Dirección Física\n", indent, val, name));
            } else if let Type::Owned(_) = ty {
                asm.push_str(&format!("{}mov rdi, {} ; Preparar tamaño para alloc\n", indent, val));
                asm.push_str(&format!("{}call sys_alloc_os ; [ALLOC] {} asume propiedad\n", indent, name));
                asm.push_str(&format!("{}mov rcx, rax ; Guardar puntero propietario\n", indent));
            } else {
                asm.push_str(&format!("{}mov rdx, {} ; [ESCALAR] {} = valor literal\n", indent, val, name));
            }
        }
        Stmt::SyntheticDrop(name) => {
            asm.push_str(&format!("\n{}// [DROP DETERMINISTA INYECTADO: {}]\n", indent, name));
            asm.push_str(&format!("{}mov rdi, rcx ; Recuperar puntero propietario\n", indent));
            asm.push_str(&format!("{}call sys_free_os ; [FREE] Liberación inmediata sin GC\n", indent));
        }
        _ => {}
    }
}
