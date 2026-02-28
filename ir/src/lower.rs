/// Lowering pass from HIR to LIR
use crate::hir::*;
use crate::lir::*;
use std::collections::HashMap;

/// Lowering context for HIR -> LIR transformation
pub struct LoweringContext {
    next_local_id: u32,
    /// Map from variable name to local index
    locals: HashMap<String, u32>,
    /// Track all locals for function signature
    local_types: Vec<LirType>,
    /// String literals collected during lowering
    string_literals: Vec<String>,
    /// Attribute offsets for the current class (name -> byte offset from object start)
    attribute_offsets: HashMap<String, u32>,
}

impl LoweringContext {
    pub fn new() -> Self {
        LoweringContext {
            next_local_id: 0,
            locals: HashMap::new(),
            local_types: Vec::new(),
            string_literals: Vec::new(),
            attribute_offsets: HashMap::new(),
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

    /// Reset context for a new function
    fn reset(&mut self) {
        self.next_local_id = 0;
        self.locals.clear();
        self.local_types.clear();
        // Don't clear attribute_offsets - they persist for the whole class
    }
    
    /// Set up attribute offsets for a class
    fn setup_attributes(&mut self, class: &HirClass) {
        self.attribute_offsets.clear();
        // Object header is 12 bytes (class_tag: 4, size: 4, vtable_ptr: 4)
        let mut offset = 12u32;
        for attr in &class.attributes {
            self.attribute_offsets.insert(attr.name.clone(), offset);
            offset += 4; // Each attribute is 4 bytes (i32 pointer)
        }
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
            
            // Set up attribute offsets for this class
            self.setup_attributes(class);

            // Lower each method to a function
            for method in &class.methods {
                let func = self.lower_method(class, method);
                functions.push(func);
            }
        }

        // Create string data section entries
        let mut string_data = Vec::new();
        for (i, s) in self.string_literals.iter().enumerate() {
            let offset = 0x2000 + (i as u32) * 128; // 128 bytes per string max
            string_data.push(StringData {
                offset,
                value: s.clone(),
            });
        }
        
        LirProgram {
            functions,
            globals: Vec::new(),
            vtables,
            string_data,
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

        // Build locals list for additional locals (beyond parameters)
        // Each local in local_types was added with the correct index already stored
        let param_count = params.len();
        let locals = self.local_types.iter().enumerate()
            .map(|(i, typ)| LirLocal {
                index: (param_count + i) as u32,
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
                // Allocate string in data section
                let str_index = self.string_literals.len();
                self.string_literals.push(value.clone());
                let offset = 0x2000 + (str_index as u32) * 128;
                
                vec![
                    LirInstr::comment(format!("String literal: {}", value)),
                    LirInstr::I32Const(offset as i32),
                ]
            }

            HirExpr::Object { name, .. } => {
                // Variable reference
                if let Some(local_id) = self.get_local(name) {
                    vec![LirInstr::LocalGet(local_id)]
                } else if let Some(&attr_offset) = self.attribute_offsets.get(name) {
                    // Attribute access: load from self at the attribute offset
                    // self is always local 0
                    vec![
                        LirInstr::comment(format!("Load attribute: {}", name)),
                        LirInstr::LocalGet(0), // self pointer
                        LirInstr::I32Load { offset: attr_offset, align: 4 },
                    ]
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
                } else if let Some(&attr_offset) = self.attribute_offsets.get(name) {
                    // Attribute assignment: self.name := expr
                    // Need to store to memory at self + attr_offset
                    // Stack has: [value]
                    // We need: self, value on stack for i32.store
                    // But we also need to return the value, so we use a temp local
                    let temp = self.alloc_local(LirType::I32);
                    instrs.push(LirInstr::LocalTee(temp)); // Save value, keep copy on stack
                    instrs.push(LirInstr::Drop); // Drop the copy (we'll reload after store)
                    instrs.push(LirInstr::LocalGet(0)); // self pointer
                    instrs.push(LirInstr::LocalGet(temp)); // value
                    instrs.push(LirInstr::I32Store { offset: attr_offset, align: 4 });
                    instrs.push(LirInstr::LocalGet(temp)); // Return the stored value
                } else {
                    instrs.push(LirInstr::comment(format!("Unknown assign target: {}", name)));
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
                // Use structured if/else for WASM compatibility
                let mut instrs = Vec::new();

                // Evaluate condition
                instrs.extend(self.lower_expr(cond));
                
                // Then and else branches lowered separately
                let then_instrs = self.lower_expr(then_branch);
                let else_instrs = self.lower_expr(else_branch);
                
                // WASM if/else always produces i32 (object pointer)
                instrs.push(LirInstr::IfElse {
                    then_instrs,
                    else_instrs,
                    result_type: Some(LirType::I32),
                });
                
                instrs
            }

            HirExpr::While { cond, body, .. } => {
                // Use structured while loop for WASM compatibility
                // WASM pattern: block { loop { cond; br_if 1; body; br 0 } } i32.const 0
                let cond_instrs = self.lower_expr(cond);
                let body_instrs = self.lower_expr(body);
                
                vec![LirInstr::WhileLoop {
                    cond_instrs,
                    body_instrs,
                }]
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
                
                // For now, use direct dispatch as we're not yet implementing vtables
                // Call the method directly: ClassName_methodName
                instrs.push(LirInstr::comment(format!(
                    "Dispatch {}.{}",
                    dispatch_info.class_name,
                    dispatch_info.method_name
                )));
                
                // Evaluate object (receiver)
                instrs.extend(self.lower_expr(object));
                
                // Evaluate arguments
                for arg in args {
                    instrs.extend(self.lower_expr(arg));
                }
                
                // Direct call for MVP
                let func_name = format!("{}_{}", dispatch_info.class_name, dispatch_info.method_name);
                instrs.push(LirInstr::Call(func_name));
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
