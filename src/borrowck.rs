use std::collections::HashMap;
use crate::ast::{Type, Stmt, Expr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceState {
    Uninitialized,
    Active(Type),
    Moved,
    Dropped,
}

#[derive(Debug)]
pub struct BorrowContext {
    /// Pila de ámbitos léxicos (Markov Blankets)
    pub scopes: Vec<HashMap<String, ResourceState>>,
    /// Manta de Markov: determina si estamos en modo de anergía controlada.
    pub is_unsafe: bool,
}

impl BorrowContext {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            is_unsafe: false,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) -> HashMap<String, ResourceState> {
        self.scopes.pop().unwrap()
    }

    pub fn declare_var(&mut self, name: String, ty: Type) {
        self.scopes.last_mut().unwrap().insert(name, ResourceState::Active(ty));
    }

    pub fn get_mut_state(&mut self, name: &str) -> Option<&mut ResourceState> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(state) = scope.get_mut(name) {
                return Some(state);
            }
        }
        None
    }

    pub fn consume_var(&mut self, name: &str) -> Result<Type, String> {
        let state_opt = self.get_mut_state(name);
        match state_opt {
            Some(state) => {
                match state {
                    ResourceState::Active(ty) => {
                        if ty.is_affine() {
                            let extracted_type = ty.clone();
                            *state = ResourceState::Moved;
                            Ok(extracted_type)
                        } else {
                            Ok(ty.clone())
                        }
                    }
                    ResourceState::Moved => Err(format!("Falsación de invariante: Uso después de Move en variable '{}'", name)),
                    ResourceState::Dropped => Err(format!("Falsación de invariante: Uso después de Drop en variable '{}'", name)),
                    ResourceState::Uninitialized => Err(format!("Uso de variable no inicializada '{}'", name)),
                }
            }
            None => Err(format!("Variable no declarada '{}'", name)),
        }
    }

    pub fn synthesize_drops_for_scope(&mut self, mut scope: HashMap<String, ResourceState>) -> Vec<Stmt> {
        let mut drops = Vec::new();
        for (name, state) in scope.iter_mut() {
            if let ResourceState::Active(ty) = state {
                if ty.is_affine() {
                    drops.push(Stmt::SyntheticDrop(name.clone()));
                    *state = ResourceState::Dropped;
                }
            }
        }
        drops
    }
}

pub fn check_statement(stmt: &Stmt, ctx: &mut BorrowContext) -> Result<Vec<Stmt>, String> {
    match stmt {
        Stmt::Let(name, ty, expr) => {
            check_expression(expr, ctx)?;
            
            if let Type::RawPtr(_) = ty {
                if !ctx.is_unsafe {
                    return Err(format!("Anergía detectada: Imposible vincular puntero físico '{}' fuera de una escotilla 'unsafe'", name));
                }
            }
            
            ctx.declare_var(name.clone(), ty.clone());
            Ok(vec![stmt.clone()])
        }
        Stmt::Block(stmts) => {
            ctx.push_scope();
            let mut transformed_stmts = Vec::new();
            for s in stmts {
                transformed_stmts.extend(check_statement(s, ctx)?);
            }
            let scope_vars = ctx.pop_scope();
            let synthetic_drops = ctx.synthesize_drops_for_scope(scope_vars);
            transformed_stmts.extend(synthetic_drops);
            
            Ok(vec![Stmt::Block(transformed_stmts)])
        }
        Stmt::Assign(_left, right) => {
            check_expression(right, ctx)?;
            Ok(vec![stmt.clone()])
        }
        Stmt::UnsafeBlock(stmts) => {
            ctx.push_scope();
            let previous_unsafe = ctx.is_unsafe;
            ctx.is_unsafe = true; // Elevación epistémica
            
            let mut transformed_stmts = Vec::new();
            for s in stmts {
                transformed_stmts.extend(check_statement(s, ctx)?);
            }
            
            ctx.is_unsafe = previous_unsafe; // Restauración de Manta de Markov
            
            let scope_vars = ctx.pop_scope();
            let synthetic_drops = ctx.synthesize_drops_for_scope(scope_vars);
            transformed_stmts.extend(synthetic_drops);
            
            Ok(vec![Stmt::UnsafeBlock(transformed_stmts)])
        }
        _ => Ok(vec![stmt.clone()]),
    }
}

pub fn check_expression(expr: &Expr, ctx: &mut BorrowContext) -> Result<Type, String> {
    match expr {
        Expr::LiteralInt(_) => Ok(Type::U64),
        Expr::LiteralBool(_) => Ok(Type::Bool),
        Expr::Variable(name) => {
            ctx.consume_var(name)
        }
        Expr::Move(name) => {
            ctx.consume_var(name)
        }
        _ => Err("Expresión no soportada aún".to_string()),
    }
}
