// ============================================================================
// SOVEREIGN COMPILER - C5-REAL ARCHITECTURE
// █ BIFURCACIÓN 2: BACKEND DE EMISIÓN DE CÓDIGO (CFG IR -> x86_64 BARE-METAL)
// ============================================================================

use crate::ir::{FunctionIr, BasicBlock, IrOp, Terminator, Operand};

/// Transductor Termodinámico: CFG SSA IR -> Código Máquina Soberano (x86_64)
pub fn generate_x86_64_from_ir(func: &FunctionIr) -> String {
    let mut asm = String::new();
    
    // Cabecera Ring-0 (no_std)
    asm.push_str("; ==========================================\n");
    asm.push_str("; SOVEREIGN COMPILER - C5-REAL ARCHITECTURE\n");
    asm.push_str("; TARGET: x86_64 Bare-Metal (Ring-0)\n");
    asm.push_str(&format!("; FUNCTION: @{}\n", func.name));
    asm.push_str("; ==========================================\n\n");
    asm.push_str(".global _start\n");
    asm.push_str(".section .text\n\n");
    
    asm.push_str("_start:\n");
    asm.push_str("    ; --- Prólogo del Kernel (Stack Frame para SSA) ---\n");
    asm.push_str("    push rbp\n");
    asm.push_str("    mov rbp, rsp\n");
    asm.push_str("    sub rsp, 512 ; Espacio contiguo para registros virtuales SSA\n\n");
    
    for bb in &func.blocks {
        emit_basic_block(bb, &mut asm);
    }
    
    asm
}

fn reg_offset(reg_idx: usize) -> usize {
    (reg_idx + 1) * 8
}

fn emit_basic_block(bb: &BasicBlock, asm: &mut String) {
    asm.push_str(&format!(".{}:\n", bb.label));
    
    for inst in &bb.instructions {
        match inst {
            IrOp::AssignImm { dest, value } => {
                asm.push_str(&format!("    ; SSA {} = Imm {}\n", dest, value));
                asm.push_str(&format!("    mov qword ptr [rbp - {}], {}\n", reg_offset(dest.0), value));
            }
            IrOp::Alloc { dest, size } => {
                asm.push_str(&format!("    ; SSA {} = Alloc {} bytes [@__sovereign_alloc]\n", dest, size));
                asm.push_str(&format!("    mov rdi, {}\n", size));
                asm.push_str("    call sys_alloc_os\n");
                asm.push_str(&format!("    mov qword ptr [rbp - {}], rax\n", reg_offset(dest.0)));
            }
            IrOp::Load { dest, ptr } => {
                asm.push_str(&format!("    ; SSA {} = Load [{}]\n", dest, ptr));
                asm.push_str(&format!("    mov rsi, qword ptr [rbp - {}]\n", reg_offset(ptr.0)));
                asm.push_str("    mov rax, qword ptr [rsi]\n");
                asm.push_str(&format!("    mov qword ptr [rbp - {}], rax\n", reg_offset(dest.0)));
            }
            IrOp::Store { ptr, value } => {
                asm.push_str(&format!("    ; Store [{}], {}\n", ptr, value));
                load_operand_to_rax(value, asm);
                asm.push_str(&format!("    mov rsi, qword ptr [rbp - {}]\n", reg_offset(ptr.0)));
                asm.push_str("    mov qword ptr [rsi], rax\n");
            }
            IrOp::VolatileLoad { dest, ptr } => {
                asm.push_str(&format!("    ; [BARRIER: VOLATILE LOAD RING-0 MMIO] SSA {} = Load [{}]\n", dest, ptr));
                asm.push_str(&format!("    mov rsi, qword ptr [rbp - {}]\n", reg_offset(ptr.0)));
                asm.push_str("    mov rax, qword ptr [rsi] ; Acceso atómico físico incondicional\n");
                asm.push_str(&format!("    mov qword ptr [rbp - {}], rax\n", reg_offset(dest.0)));
            }
            IrOp::VolatileStore { ptr, value } => {
                asm.push_str("    ; [BARRIER: VOLATILE STORE RING-0 MMIO - IMMUNE TO DCE]\n");
                load_operand_to_rax(value, asm);
                asm.push_str(&format!("    mov rsi, qword ptr [rbp - {}]\n", reg_offset(ptr.0)));
                asm.push_str("    mov qword ptr [rsi], rax ; Escritura física directa al bus de hardware\n");
                asm.push_str("    mfence ; Barrera de serialización estricta de memoria\n");
            }
            IrOp::Free { ptr } => {
                asm.push_str(&format!("    ; [DEALLOC DETERMINISTA MATERIALIZADO: @__sovereign_dealloc({})]\n", ptr));
                asm.push_str(&format!("    mov rdi, qword ptr [rbp - {}]\n", reg_offset(ptr.0)));
                asm.push_str("    call sys_free_os\n");
            }
            IrOp::Add { dest, lhs, rhs } => {
                asm.push_str(&format!("    ; SSA {} = Add {}, {}\n", dest, lhs, rhs));
                load_operand_to_rax(lhs, asm);
                match rhs {
                    Operand::Imm(v) => asm.push_str(&format!("    add rax, {}\n", v)),
                    Operand::Reg(r) => asm.push_str(&format!("    add rax, qword ptr [rbp - {}]\n", reg_offset(r.0))),
                }
                asm.push_str(&format!("    mov qword ptr [rbp - {}], rax\n", reg_offset(dest.0)));
            }
            IrOp::Call { dest, target, args } => {
                asm.push_str(&format!("    ; Call {}({:?})\n", target, args));
                asm.push_str(&format!("    call {}\n", target));
                if let Some(d) = dest {
                    asm.push_str(&format!("    mov qword ptr [rbp - {}], rax\n", reg_offset(d.0)));
                }
            }
        }
    }
    
    // Emisión del terminador del bloque básico
    match &bb.terminator {
        Terminator::Jump(target) => {
            asm.push_str(&format!("    jmp .{}\n\n", target));
        }
        Terminator::BranchIf { cond, then_bb, else_bb } => {
            asm.push_str(&format!("    mov rax, qword ptr [rbp - {}]\n", reg_offset(cond.0)));
            asm.push_str("    test rax, rax\n");
            asm.push_str(&format!("    jnz .{}\n", then_bb));
            asm.push_str(&format!("    jmp .{}\n\n", else_bb));
        }
        Terminator::Return(Some(op)) => {
            load_operand_to_rax(op, asm);
            asm.push_str("    mov rsp, rbp\n");
            asm.push_str("    pop rbp\n");
            asm.push_str("    ret\n\n");
        }
        Terminator::Return(None) => {
            asm.push_str("    mov rsp, rbp\n");
            asm.push_str("    pop rbp\n");
            asm.push_str("    ret\n\n");
        }
        Terminator::Halt => {
            asm.push_str("    ; --- Kernel Panic / Halt Loop (Ring-0) ---\n");
            asm.push_str(&format!(".{}_halt:\n", bb.label));
            asm.push_str("    cli\n");
            asm.push_str("    hlt\n");
            asm.push_str(&format!("    jmp .{}_halt\n\n", bb.label));
        }
    }
}

fn load_operand_to_rax(op: &Operand, asm: &mut String) {
    match op {
        Operand::Imm(v) => asm.push_str(&format!("    mov rax, {}\n", v)),
        Operand::Reg(r) => asm.push_str(&format!("    mov rax, qword ptr [rbp - {}]\n", reg_offset(r.0))),
    }
}
