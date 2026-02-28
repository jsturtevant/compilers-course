/// Lowering pass from HIR to LIR
use crate::hir::*;
use crate::lir::*;
use std::collections::HashMap;

/// Lowering context for HIR -> LIR transformation
pub struct LoweringContext {
    next_local_id: u32,
    next_label_id: usize,
    /// Map from variable name to local index
    locals: HashMap<String, u32>,
    /// Track all locals for function signature
    local_types: Vec<LirType>,
}

impl LoweringContext {
    pub fn new() -> Self {
        LoweringContext {
            next_local_id: 0,
            next_label_id: 0,
            locals: HashMap::new(),
            local_types: Vec::new(),
        }
    }

    /// Allocate a new local variable
    fn alloc_local(&mut self, typ: LirType) -> u32 {
        let id = self.next_local_id;
        self.next_local_id += 1;
        self.local_types.push(typ);
        id
    }

    /// Register a named local variable
    fn register_local(&mut self, name: String, typ: LirType) -> u32 {
        let id = self.alloc_local(typ);
        self.locals.insert(name, id);
        id
    }

    /// Get local variable index by name
    fn get_local(&self, name: &str) -> Option<u32> {
        self.locals.get(name).copied()
    }

    /// Generate a fresh label
    fn fresh_label(&mut self, prefix: &str) -> Label {
        let id = self.next_label_id;
        self.next_label_id += 1;
        Label::fresh(prefix, id)
    }

    /// Reset context for a new function
    fn reset(&mut self) {
        self.next_local_id = 0;
        self.locals.clear();
        self.local_types.clear();
    }

    /// Lower a HIR program to LIR
    pub fn lower_program(&mut self, program: &HirProgram) -> LirProgram {
        let mut functions = Vec::new();
        let mut vtables = Vec::new();

        // Lower each class
        for class in &program.classes {
            // Generate vtable for this class
            let vtable = self.lower_vtable(class);
            vtables.push(vtable);

            // Lower each method to a function
            for method in &class.methods {
                let func = self.lower_method(class, method);
                functions.push(func);
            }
        }

        // TODO: Add built-in runtime functions (IO, String operations, etc.)
        
        LirProgram {
            functions,
            globals: Vec::new(),
            vtables,
        }
    }

    /// Lower a class's vtable
    fn lower_vtable(&self, class: &HirClass) -> VTable {
        let methods = class.methods.iter()
            .map(|m| format!("{}_{}", class.name, m.name))
            .collect();

        VTable {
            class_name: class.name.clone(),
            methods,
        }
    }

    /// Lower a method to a LIR function
    fn lower_method(&mut self, class: &HirClass, method: &HirMethod) -> LirFunction {
        // Reset context for each function
        self.reset();

        // Parameters: self + formals
        let mut params = vec![LirType::I32]; // self pointer
        self.locals.insert("self".to_string(), 0);
        self.next_local_id = 1;

        for (i, formal) in method.formals.iter().enumerate() {
            params.push(LirType::I32); // All COOL values are i32 (pointers or immediate)
            self.locals.insert(formal.name.clone(), (i as u32) + 1);
            self.next_local_id += 1;
        }

        // Return type
        let return_type = match method.return_type {
            TypeId::NoType => LirType::Void,
            _ => LirType::I32,
        };

        // Function name: ClassName_methodName
        let name = format!("{}_{}", class.name, method.name);

        // Lower method body
        let body = self.lower_expr(&method.body);

        // Build locals list from tracked types
        let locals = self.local_types.iter().enumerate()
            .map(|(i, typ)| LirLocal {
                index: i as u32,
                typ: typ.clone(),
            })
            .collect();

        LirFunction {
            name,
            params,
            return_type,
            locals,
            body,
        }
    }

    /// Lower a HIR expression to LIR instructions
    fn lower_expr(&mut self, expr: &HirExpr) -> Vec<LirInstr> {
        match expr {
            HirExpr::IntLiteral { value, .. } => {
                vec![LirInstr::I32Const(*value)]
            }

            HirExpr::BoolLiteral { value, .. } => {
                vec![LirInstr::I32Const(if *value { 1 } else { 0 })]
            }

            HirExpr::StringLiteral { value, .. } => {
                // TODO: Allocate string object in data section, return pointer
                vec![
                    LirInstr::comment(format!("String literal: {}", value)),
                    LirInstr::I32Const(0), // Placeholder: return null for now
                ]
            }

            HirExpr::Object { name, .. } => {
                // Variable reference
                if let Some(local_id) = self.get_local(name) {
                    vec![LirInstr::LocalGet(local_id)]
                } else {
                    vec![
                        LirInstr::comment(format!("Unknown variable: {}", name)),
                        LirInstr::I32Const(0),
                    ]
                }
            }

            HirExpr::Assign { name, expr, .. } => {
                let mut instrs = Vec::new();
                // Evaluate expression
                instrs.extend(self.lower_expr(expr));
                // Store to local (or attribute if not local)
                if let Some(local_id) = self.get_local(name) {
                    instrs.push(LirInstr::LocalTee(local_id));
                } else {
                    // Attribute assignment: self.name := expr
                    // TODO: Calculate attribute offset and store to memory
                    instrs.push(LirInstr::comment(format!("Assign to attribute: {}", name)));
                }
                instrs
            }

            HirExpr::Add { left, right, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(left));
                instrs.extend(self.lower_expr(right));
                instrs.push(LirInstr::I32Add);
                instrs
            }

            HirExpr::Sub { left, right, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(left));
                instrs.extend(self.lower_expr(right));
                instrs.push(LirInstr::I32Sub);
                instrs
            }

            HirExpr::Mul { left, right, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(left));
                instrs.extend(self.lower_expr(right));
                instrs.push(LirInstr::I32Mul);
                instrs
            }

            HirExpr::Div { left, right, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(left));
                instrs.extend(self.lower_expr(right));
                // TODO: Check for division by zero
                instrs.push(LirInstr::I32DivS);
                instrs
            }

            HirExpr::Lt { left, right, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(left));
                instrs.extend(self.lower_expr(right));
                instrs.push(LirInstr::I32LtS);
                instrs
            }

            HirExpr::Le { left, right, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(left));
                instrs.extend(self.lower_expr(right));
                instrs.push(LirInstr::I32LeS);
                instrs
            }

            HirExpr::Eq { left, right, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(left));
                instrs.extend(self.lower_expr(right));
                instrs.push(LirInstr::I32Eq);
                instrs
            }

            HirExpr::Not { expr, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(expr));
                instrs.push(LirInstr::I32Eqz); // Boolean not = compare to zero
                instrs
            }

            HirExpr::Negate { expr, .. } => {
                let mut instrs = Vec::new();
                instrs.push(LirInstr::I32Const(0));
                instrs.extend(self.lower_expr(expr));
                instrs.push(LirInstr::I32Sub); // 0 - expr
                instrs
            }

            HirExpr::IsVoid { expr, .. } => {
                let mut instrs = Vec::new();
                instrs.extend(self.lower_expr(expr));
                instrs.push(LirInstr::I32Eqz); // Check if null (0)
                instrs
            }

            HirExpr::Block { exprs, .. } => {
                let mut instrs = Vec::new();
                for (i, e) in exprs.iter().enumerate() {
                    instrs.extend(self.lower_expr(e));
                    // Drop all but the last expression result
                    if i < exprs.len() - 1 {
                        instrs.push(LirInstr::Drop);
                    }
                }
                instrs
            }

            HirExpr::If { cond, then_branch, else_branch, .. } => {
                let mut instrs = Vec::new();
                let else_label = self.fresh_label("else");
                let end_label = self.fresh_label("endif");

                // Evaluate condition
                instrs.extend(self.lower_expr(cond));
                
                // Jump to else if false
                instrs.push(LirInstr::I32Eqz);
                instrs.push(LirInstr::JumpIf(else_label.clone()));
                
                // Then branch
                instrs.extend(self.lower_expr(then_branch));
                instrs.push(LirInstr::Jump(end_label.clone()));
                
                // Else branch
                instrs.push(LirInstr::Label(else_label));
                instrs.extend(self.lower_expr(else_branch));
                
                // End
                instrs.push(LirInstr::Label(end_label));
                instrs
            }

            HirExpr::While { cond, body, .. } => {
                let mut instrs = Vec::new();
                let loop_label = self.fresh_label("loop");
                let end_label = self.fresh_label("endloop");

                // Loop start
                instrs.push(LirInstr::Label(loop_label.clone()));
                
                // Evaluate condition
                instrs.extend(self.lower_expr(cond));
                
                // Exit if false
                instrs.push(LirInstr::I32Eqz);
                instrs.push(LirInstr::JumpIf(end_label.clone()));
                
                // Loop body
                instrs.extend(self.lower_expr(body));
                instrs.push(LirInstr::Drop); // Discard body result
                
                // Jump back to loop start
                instrs.push(LirInstr::Jump(loop_label));
                
                // End (while always returns void/0)
                instrs.push(LirInstr::Label(end_label));
                instrs.push(LirInstr::I32Const(0));
                instrs
            }

            HirExpr::Let { name, init, body, .. } => {
                let mut instrs = Vec::new();
                
                // Evaluate init expression
                instrs.extend(self.lower_expr(init));
                
                // Allocate local for let binding
                let local_id = self.register_local(name.clone(), LirType::I32);
                instrs.push(LirInstr::LocalSet(local_id));
                
                // Evaluate body with binding in scope
                instrs.extend(self.lower_expr(body));
                
                // Note: In real implementation, would need to pop local scope
                instrs
            }

            HirExpr::New { type_name, .. } => {
                let mut instrs = Vec::new();
                // TODO: Calculate object size, allocate memory, initialize
                instrs.push(LirInstr::comment(format!("new {}", type_name)));
                instrs.push(LirInstr::I32Const(16)); // Placeholder size
                instrs.push(LirInstr::Alloc);
                instrs
            }

            HirExpr::Dispatch { object, dispatch_info, args, .. } => {
                let mut instrs = Vec::new();
                
                // Evaluate object (receiver)
                instrs.extend(self.lower_expr(object));
                
                // Evaluate arguments
                for arg in args {
                    instrs.extend(self.lower_expr(arg));
                }
                
                // Load vtable and call indirect
                instrs.push(LirInstr::comment(format!(
                    "Dispatch {}.{}",
                    dispatch_info.class_name,
                    dispatch_info.method_name
                )));
                
                // TODO: Load vtable pointer from object, index into vtable
                instrs.push(LirInstr::CallIndirect {
                    type_index: 0, // TODO: Calculate type index
                    method_index: dispatch_info.method_index as u32,
                });
                instrs
            }

            HirExpr::StaticDispatch { object, type_name, dispatch_info, args, .. } => {
                let mut instrs = Vec::new();
                
                // Evaluate object (receiver)
                instrs.extend(self.lower_expr(object));
                
                // Evaluate arguments
                for arg in args {
                    instrs.extend(self.lower_expr(arg));
                }
                
                // Static dispatch: direct call to Class.method
                let func_name = format!("{}_{}", type_name, dispatch_info.method_name);
                instrs.push(LirInstr::Call(func_name));
                instrs
            }

            HirExpr::Case { expr, branches, .. } => {
                let mut instrs = Vec::new();
                
                // Evaluate case expression
                instrs.extend(self.lower_expr(expr));
                
                // TODO: Implement type-based dispatch for case branches
                // For now, just execute first branch
                instrs.push(LirInstr::comment("Case expression (simplified)"));
                if let Some(first_branch) = branches.first() {
                    let local_id = self.register_local(first_branch.name.clone(), LirType::I32);
                    instrs.push(LirInstr::LocalSet(local_id));
                    instrs.extend(self.lower_expr(&first_branch.expr));
                }
                instrs
            }
        }
    }
}

impl Default for LoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lower_int_literal() {
        let mut ctx = LoweringContext::new();
        let expr = HirExpr::IntLiteral {
            value: 42,
            typ: TypeId::Int,
        };
        let instrs = ctx.lower_expr(&expr);
        assert_eq!(instrs.len(), 1);
        match &instrs[0] {
            LirInstr::I32Const(val) => assert_eq!(*val, 42),
            _ => panic!("Expected I32Const"),
        }
    }

    #[test]
    fn test_lower_arithmetic() {
        let mut ctx = LoweringContext::new();
        let expr = HirExpr::Add {
            left: Box::new(HirExpr::IntLiteral {
                value: 1,
                typ: TypeId::Int,
            }),
            right: Box::new(HirExpr::IntLiteral {
                value: 2,
                typ: TypeId::Int,
            }),
            typ: TypeId::Int,
        };
        let instrs = ctx.lower_expr(&expr);
        assert_eq!(instrs.len(), 3);
        assert!(matches!(instrs[0], LirInstr::I32Const(1)));
        assert!(matches!(instrs[1], LirInstr::I32Const(2)));
        assert!(matches!(instrs[2], LirInstr::I32Add));
    }
}
