/// Lowering pass from HIR to LIR
use crate::hir::*;
use crate::lir::*;
use std::collections::HashMap;

/// Information about a class needed for object allocation
#[derive(Debug, Clone)]
pub struct ClassMetadata {
    pub class_tag: usize,
    pub object_size: u32,  // Total size in bytes (header + attributes)
    pub attributes: Vec<(String, TypeId, u32)>,  // (name, type, offset)
}

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
    /// Map from class name to class tag (for case dispatch)
    class_tags: HashMap<String, usize>,
    /// Map from class name to parent class name (for inheritance checking)
    class_parents: HashMap<String, Option<String>>,
    /// Map from class name to class metadata (for new expression)
    class_metadata: HashMap<String, ClassMetadata>,
}

impl LoweringContext {
    pub fn new() -> Self {
        LoweringContext {
            next_local_id: 0,
            locals: HashMap::new(),
            local_types: Vec::new(),
            string_literals: Vec::new(),
            attribute_offsets: HashMap::new(),
            class_tags: HashMap::new(),
            class_parents: HashMap::new(),
            class_metadata: HashMap::new(),
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

        // Build class tag and parent maps for case dispatch
        for class in &program.classes {
            self.class_tags.insert(class.name.clone(), class.class_tag);
            self.class_parents.insert(class.name.clone(), class.parent.clone());
            
            // Build class metadata for new expression
            let mut offset = 12u32; // Object header is 12 bytes
            let mut attrs = Vec::new();
            for attr in &class.attributes {
                attrs.push((attr.name.clone(), attr.typ.clone(), offset));
                offset += 4; // Each attribute is 4 bytes
            }
            self.class_metadata.insert(class.name.clone(), ClassMetadata {
                class_tag: class.class_tag,
                object_size: offset, // 12 + 4*num_attributes
                attributes: attrs,
            });
        }
        // Add built-in classes with their tags (Object=0, IO=1, String=2, Int=3, Bool=4)
        // These may not be in the program but are needed for case matching
        if !self.class_tags.contains_key("Object") {
            self.class_tags.insert("Object".to_string(), 0);
            self.class_parents.insert("Object".to_string(), None);
            self.class_metadata.insert("Object".to_string(), ClassMetadata {
                class_tag: 0,
                object_size: 12, // Just header, no attributes
                attributes: Vec::new(),
            });
        }
        if !self.class_tags.contains_key("IO") {
            self.class_tags.insert("IO".to_string(), 1);
            self.class_parents.insert("IO".to_string(), Some("Object".to_string()));
            self.class_metadata.insert("IO".to_string(), ClassMetadata {
                class_tag: 1,
                object_size: 12,
                attributes: Vec::new(),
            });
        }
        if !self.class_tags.contains_key("String") {
            self.class_tags.insert("String".to_string(), 2);
            self.class_parents.insert("String".to_string(), Some("Object".to_string()));
            // String has special layout: header + length + data pointer
            self.class_metadata.insert("String".to_string(), ClassMetadata {
                class_tag: 2,
                object_size: 20, // 12 header + 4 length + 4 data ptr
                attributes: vec![
                    ("length".to_string(), TypeId::Int, 12),
                    ("data".to_string(), TypeId::Int, 16),
                ],
            });
        }
        if !self.class_tags.contains_key("Int") {
            self.class_tags.insert("Int".to_string(), 3);
            self.class_parents.insert("Int".to_string(), Some("Object".to_string()));
            self.class_metadata.insert("Int".to_string(), ClassMetadata {
                class_tag: 3,
                object_size: 16, // 12 header + 4 value
                attributes: vec![("value".to_string(), TypeId::Int, 12)],
            });
        }
        if !self.class_tags.contains_key("Bool") {
            self.class_tags.insert("Bool".to_string(), 4);
            self.class_parents.insert("Bool".to_string(), Some("Object".to_string()));
            self.class_metadata.insert("Bool".to_string(), ClassMetadata {
                class_tag: 4,
                object_size: 16, // 12 header + 4 value
                attributes: vec![("value".to_string(), TypeId::Bool, 12)],
            });
        }

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
                instrs.push(LirInstr::comment(format!("new {}", type_name)));
                
                // Look up class metadata
                if let Some(metadata) = self.class_metadata.get(type_name).cloned() {
                    // Allocate a temporary local to hold the object pointer
                    // alloc_local returns the correct WASM local index
                    let obj_local = self.alloc_local(LirType::I32);
                    
                    // Allocate memory for the object
                    instrs.push(LirInstr::I32Const(metadata.object_size as i32));
                    instrs.push(LirInstr::Alloc);
                    instrs.push(LirInstr::LocalTee(obj_local));
                    
                    // Store class tag at offset 0
                    instrs.push(LirInstr::I32Const(metadata.class_tag as i32));
                    instrs.push(LirInstr::I32Store { offset: 0, align: 4 });
                    
                    // Store object size at offset 4
                    instrs.push(LirInstr::LocalGet(obj_local));
                    instrs.push(LirInstr::I32Const(metadata.object_size as i32));
                    instrs.push(LirInstr::I32Store { offset: 4, align: 4 });
                    
                    // Store vtable pointer at offset 8 (0 for now, vtables not fully implemented)
                    instrs.push(LirInstr::LocalGet(obj_local));
                    instrs.push(LirInstr::I32Const(0)); // TODO: vtable address
                    instrs.push(LirInstr::I32Store { offset: 8, align: 4 });
                    
                    // Initialize attributes to default values based on type
                    for (attr_name, attr_type, attr_offset) in &metadata.attributes {
                        instrs.push(LirInstr::comment(format!("init attr {}: {:?}", attr_name, attr_type)));
                        instrs.push(LirInstr::LocalGet(obj_local));
                        
                        // Default value based on type
                        let default_value = match attr_type {
                            TypeId::Int => 0,      // Int defaults to 0
                            TypeId::Bool => 0,     // Bool defaults to false
                            TypeId::String => 0,   // String defaults to null (empty string ptr)
                            _ => 0,                // Reference types default to null (0)
                        };
                        instrs.push(LirInstr::I32Const(default_value));
                        instrs.push(LirInstr::I32Store { offset: *attr_offset, align: 4 });
                    }
                    
                    // Leave the object pointer on the stack as the result
                    instrs.push(LirInstr::LocalGet(obj_local));
                } else {
                    // Fallback for unknown types (shouldn't happen with proper semantic analysis)
                    instrs.push(LirInstr::comment(format!("WARNING: unknown type {}", type_name)));
                    instrs.push(LirInstr::I32Const(12)); // Minimum object size (header only)
                    instrs.push(LirInstr::Alloc);
                }
                
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
                
                // Evaluate case expression - get object pointer on stack
                instrs.extend(self.lower_expr(expr));
                
                // Store object pointer in a local for reuse
                let obj_local = self.alloc_local(LirType::I32);
                instrs.push(LirInstr::LocalTee(obj_local));
                
                // Read class tag from object (at offset 0)
                instrs.push(LirInstr::I32Load { offset: 0, align: 4 });
                let tag_local = self.alloc_local(LirType::I32);
                instrs.push(LirInstr::LocalSet(tag_local));
                
                instrs.push(LirInstr::comment("Case expression - type dispatch"));
                
                if branches.is_empty() {
                    // No branches - abort
                    instrs.push(LirInstr::Unreachable);
                    return instrs;
                }
                
                // Sort branches by specificity (more derived types first)
                // We sort by inheritance depth (deeper = more specific)
                let mut sorted_branches: Vec<_> = branches.iter().collect();
                sorted_branches.sort_by(|a, b| {
                    let depth_a = self.get_inheritance_depth(&a.type_name);
                    let depth_b = self.get_inheritance_depth(&b.type_name);
                    depth_b.cmp(&depth_a) // Reverse order: deeper first
                });
                
                // Get all conforming tags for each branch type
                // For each branch type, collect all class tags that conform to it
                let branch_tag_sets: Vec<(_, Vec<usize>)> = sorted_branches.iter()
                    .map(|branch| {
                        let conforming_tags = self.get_conforming_tags(&branch.type_name);
                        (*branch, conforming_tags)
                    })
                    .collect();
                
                // Generate nested if/else chain
                // Structure: if (tag matches branch1) { body1 } else if (tag matches branch2) { body2 } else { abort }
                let case_instrs = self.build_case_dispatch(
                    obj_local,
                    tag_local,
                    &branch_tag_sets,
                );
                
                instrs.extend(case_instrs);
                instrs
            }
        }
    }
    
    /// Get inheritance depth (Object = 0, IO = 1, etc.)
    fn get_inheritance_depth(&self, class_name: &str) -> usize {
        let mut depth = 0;
        let mut current = class_name.to_string();
        while let Some(parent) = self.class_parents.get(&current).and_then(|p| p.clone()) {
            depth += 1;
            current = parent;
        }
        depth
    }
    
    /// Get all class tags that conform to (are subtypes of) the given type
    fn get_conforming_tags(&self, type_name: &str) -> Vec<usize> {
        let mut tags = Vec::new();
        
        // Check all known classes to see if they conform to type_name
        for (class_name, &tag) in &self.class_tags {
            if self.class_conforms(class_name, type_name) {
                tags.push(tag);
            }
        }
        
        tags
    }
    
    /// Check if class1 conforms to class2 (class1 is subtype of class2)
    fn class_conforms(&self, class1: &str, class2: &str) -> bool {
        if class1 == class2 {
            return true;
        }
        
        let mut current = class1.to_string();
        while let Some(parent) = self.class_parents.get(&current).and_then(|p| p.clone()) {
            if parent == class2 {
                return true;
            }
            current = parent;
        }
        
        false
    }
    
    /// Build case dispatch as nested IfElse instructions
    fn build_case_dispatch(
        &mut self,
        obj_local: u32,
        tag_local: u32,
        branches: &[(&CaseBranch, Vec<usize>)],
    ) -> Vec<LirInstr> {
        if branches.is_empty() {
            // No match - call abort
            return vec![
                LirInstr::comment("Case no-match: abort"),
                LirInstr::Unreachable,
            ];
        }
        
        let (branch, conforming_tags) = &branches[0];
        let remaining = &branches[1..];
        
        // Build condition: check if tag matches any conforming tag
        // (tag == tag1) || (tag == tag2) || ...
        let cond_instrs = self.build_tag_check_condition(tag_local, conforming_tags);
        
        // Build then branch: bind variable and execute body
        let mut then_instrs = Vec::new();
        // Bind the case variable to the object
        let var_local = self.register_local(branch.name.clone(), LirType::I32);
        then_instrs.push(LirInstr::LocalGet(obj_local));
        then_instrs.push(LirInstr::LocalSet(var_local));
        // Execute branch body
        then_instrs.extend(self.lower_expr(&branch.expr));
        
        // Build else branch: try remaining branches
        let else_instrs = self.build_case_dispatch(obj_local, tag_local, remaining);
        
        // Combine into IfElse
        let mut result = cond_instrs;
        result.push(LirInstr::IfElse {
            then_instrs,
            else_instrs,
            result_type: Some(LirType::I32), // Case always returns i32 (object pointer)
        });
        
        result
    }
    
    /// Build condition checking if tag matches any of the given tags
    fn build_tag_check_condition(&self, tag_local: u32, tags: &[usize]) -> Vec<LirInstr> {
        if tags.is_empty() {
            return vec![LirInstr::I32Const(0)]; // Never matches
        }
        
        let mut instrs = Vec::new();
        
        // Build: (tag == tags[0]) || (tag == tags[1]) || ...
        // In WASM: we compute each comparison and OR them together
        
        // First comparison
        instrs.push(LirInstr::LocalGet(tag_local));
        instrs.push(LirInstr::I32Const(tags[0] as i32));
        instrs.push(LirInstr::I32Eq);
        
        // Additional comparisons with OR
        for &tag in &tags[1..] {
            instrs.push(LirInstr::LocalGet(tag_local));
            instrs.push(LirInstr::I32Const(tag as i32));
            instrs.push(LirInstr::I32Eq);
            // OR with previous result: (a || b) = (a + b) > 0, or use i32.or
            // Actually for booleans, we can use i32.or
            instrs.push(LirInstr::I32Or);
        }
        
        instrs
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
