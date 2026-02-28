/// Lowering pass from HIR to LIR
use crate::hir::*;
use crate::lir::*;
use std::collections::HashMap;

/// Information about a class needed for object allocation
#[derive(Debug, Clone)]
pub struct ClassMetadata {
    pub class_tag: usize,
    pub object_size: u32,  // Total size in bytes (header + attributes)
    pub attributes: Vec<(String, TypeId, u32, Option<HirExpr>)>,  // (name, type, offset, initializer)
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
    /// Map from class name to vtable (list of function names)
    class_vtables: HashMap<String, Vec<String>>,
    /// Map from class name to methods defined in that class
    class_methods: HashMap<String, Vec<String>>,
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
            class_vtables: HashMap::new(),
            class_methods: HashMap::new(),
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
    
    /// Set up attribute offsets for a class (including inherited attributes)
    fn setup_attributes(&mut self, class: &HirClass, program: &HirProgram) {
        self.attribute_offsets.clear();
        // Object header is 12 bytes (class_tag: 4, size: 4, vtable_ptr: 4)
        let mut offset = 12u32;
        
        // First, get inherited attributes by walking up the parent chain
        let mut ancestor_attrs: Vec<(String, u32)> = Vec::new();
        let mut current_parent = class.parent.clone();
        while let Some(ref parent_name) = current_parent {
            // Skip builtin classes (Object, IO, String, Int, Bool - they don't have user attributes)
            if matches!(parent_name.as_str(), "Object" | "IO" | "String" | "Int" | "Bool") {
                break;
            }
            // Find the parent class in the program
            if let Some(parent_class) = program.classes.iter().find(|c| &c.name == parent_name) {
                // Prepend parent's attributes (they come first in memory layout)
                for attr in parent_class.attributes.iter().rev() {
                    ancestor_attrs.insert(0, (attr.name.clone(), 0)); // offset will be calculated
                }
                current_parent = parent_class.parent.clone();
            } else {
                break;
            }
        }
        
        // Add inherited attributes first (with calculated offsets)
        for (attr_name, _) in &ancestor_attrs {
            self.attribute_offsets.insert(attr_name.clone(), offset);
            offset += 4;
        }
        
        // Then add this class's own attributes
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
            
            // Track which methods are defined in this class
            let methods: Vec<String> = class.methods.iter()
                .map(|m| m.name.clone())
                .collect();
            self.class_methods.insert(class.name.clone(), methods);
        }
        
        // Build class metadata for new expression (need second pass to resolve inherited attrs)
        for class in &program.classes {
            let mut offset = 12u32; // Object header is 12 bytes
            let mut attrs = Vec::new();
            
            // First, collect inherited attributes by walking up the parent chain
            let mut ancestor_attrs: Vec<HirAttribute> = Vec::new();
            let mut current_parent = class.parent.clone();
            while let Some(ref parent_name) = current_parent {
                // Skip builtin classes
                if matches!(parent_name.as_str(), "Object" | "IO" | "String" | "Int" | "Bool") {
                    break;
                }
                if let Some(parent_class) = program.classes.iter().find(|c| &c.name == parent_name) {
                    // Prepend parent's attributes
                    for attr in parent_class.attributes.iter().rev() {
                        ancestor_attrs.insert(0, attr.clone());
                    }
                    current_parent = parent_class.parent.clone();
                } else {
                    break;
                }
            }
            
            // Add inherited attributes first
            for attr in &ancestor_attrs {
                attrs.push((attr.name.clone(), attr.typ.clone(), offset, attr.init.clone()));
                offset += 4;
            }
            
            // Then add this class's own attributes
            for attr in &class.attributes {
                attrs.push((attr.name.clone(), attr.typ.clone(), offset, attr.init.clone()));
                offset += 4; // Each attribute is 4 bytes
            }
            
            self.class_metadata.insert(class.name.clone(), ClassMetadata {
                class_tag: class.class_tag,
                object_size: offset, // 12 + 4*num_attributes (including inherited)
                attributes: attrs,
            });
        }
        // Add built-in classes with their tags (Object=0, IO=1, String=2, Int=3, Bool=4)
        // These may not be in the program but are needed for case matching
        // NOTE: Method order must match get_all_methods() in class_hierarchy.rs
        // which sorts methods alphabetically within each class
        if !self.class_tags.contains_key("Object") {
            self.class_tags.insert("Object".to_string(), 0);
            self.class_parents.insert("Object".to_string(), None);
            self.class_methods.insert("Object".to_string(), vec!["abort".to_string(), "copy".to_string(), "type_name".to_string()]);
            self.class_vtables.insert("Object".to_string(), vec![
                "Object_abort".to_string(),
                "Object_copy".to_string(),
                "Object_type_name".to_string(),
            ]);
            self.class_metadata.insert("Object".to_string(), ClassMetadata {
                class_tag: 0,
                object_size: 12, // Just header, no attributes
                attributes: Vec::new(),
            });
        }
        if !self.class_tags.contains_key("IO") {
            self.class_tags.insert("IO".to_string(), 1);
            self.class_parents.insert("IO".to_string(), Some("Object".to_string()));
            self.class_methods.insert("IO".to_string(), vec![
                "in_int".to_string(), "in_string".to_string(),
                "out_int".to_string(), "out_string".to_string(),
            ]);
            self.class_vtables.insert("IO".to_string(), vec![
                "Object_abort".to_string(),
                "Object_copy".to_string(),
                "Object_type_name".to_string(),
                "IO_in_int".to_string(),
                "IO_in_string".to_string(),
                "IO_out_int".to_string(),
                "IO_out_string".to_string(),
            ]);
            self.class_metadata.insert("IO".to_string(), ClassMetadata {
                class_tag: 1,
                object_size: 12,
                attributes: Vec::new(),
            });
        }
        if !self.class_tags.contains_key("String") {
            self.class_tags.insert("String".to_string(), 2);
            self.class_parents.insert("String".to_string(), Some("Object".to_string()));
            // Alphabetical order: concat, length, substr
            self.class_methods.insert("String".to_string(), vec![
                "concat".to_string(), "length".to_string(), "substr".to_string(),
            ]);
            self.class_vtables.insert("String".to_string(), vec![
                "Object_abort".to_string(),
                "Object_copy".to_string(),
                "Object_type_name".to_string(),
                "String_concat".to_string(),
                "String_length".to_string(),
                "String_substr".to_string(),
            ]);
            // String has special layout: header + length + data pointer
            self.class_metadata.insert("String".to_string(), ClassMetadata {
                class_tag: 2,
                object_size: 20, // 12 header + 4 length + 4 data ptr
                attributes: vec![
                    ("length".to_string(), TypeId::Int, 12, None),
                    ("data".to_string(), TypeId::Int, 16, None),
                ],
            });
        }
        if !self.class_tags.contains_key("Int") {
            self.class_tags.insert("Int".to_string(), 3);
            self.class_parents.insert("Int".to_string(), Some("Object".to_string()));
            self.class_methods.insert("Int".to_string(), vec![]);
            self.class_vtables.insert("Int".to_string(), vec![
                "Object_abort".to_string(),
                "Object_copy".to_string(),
                "Object_type_name".to_string(),
            ]);
            self.class_metadata.insert("Int".to_string(), ClassMetadata {
                class_tag: 3,
                object_size: 16, // 12 header + 4 value
                attributes: vec![("value".to_string(), TypeId::Int, 12, None)],
            });
        }
        if !self.class_tags.contains_key("Bool") {
            self.class_tags.insert("Bool".to_string(), 4);
            self.class_parents.insert("Bool".to_string(), Some("Object".to_string()));
            self.class_methods.insert("Bool".to_string(), vec![]);
            self.class_vtables.insert("Bool".to_string(), vec![
                "Object_abort".to_string(),
                "Object_copy".to_string(),
                "Object_type_name".to_string(),
            ]);
            self.class_metadata.insert("Bool".to_string(), ClassMetadata {
                class_tag: 4,
                object_size: 16, // 12 header + 4 value
                attributes: vec![("value".to_string(), TypeId::Bool, 12, None)],
            });
        }

        // Build vtables for user-defined classes with proper inheritance
        // Process classes - we need to process in inheritance order
        let sorted_classes = self.sort_classes_by_inheritance(program);
        for class_name in &sorted_classes {
            let class = program.classes.iter().find(|c| &c.name == class_name).unwrap();
            let vtable = self.build_vtable_with_inheritance(class);
            self.class_vtables.insert(class.name.clone(), vtable.clone());
        }
        
        // Now generate LIR vtables
        for class in &program.classes {
            if let Some(methods) = self.class_vtables.get(&class.name) {
                vtables.push(VTable {
                    class_name: class.name.clone(),
                    methods: methods.clone(),
                });
            }
        }
        
        // Lower each class
        for class in &program.classes {
            // Set up attribute offsets for this class (including inherited)
            self.setup_attributes(class, program);

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
    
    /// Sort classes so parents come before children
    fn sort_classes_by_inheritance(&self, program: &HirProgram) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        
        fn visit(
            class_name: &str,
            parents: &HashMap<String, Option<String>>,
            program: &HirProgram,
            visited: &mut std::collections::HashSet<String>,
            result: &mut Vec<String>,
        ) {
            if visited.contains(class_name) {
                return;
            }
            
            // Visit parent first
            if let Some(Some(parent)) = parents.get(class_name) {
                // Only visit if it's a user-defined class
                if program.classes.iter().any(|c| &c.name == parent) {
                    visit(parent, parents, program, visited, result);
                }
            }
            
            visited.insert(class_name.to_string());
            result.push(class_name.to_string());
        }
        
        for class in &program.classes {
            visit(&class.name, &self.class_parents, program, &mut visited, &mut result);
        }
        
        result
    }
    
    /// Build vtable for a class including inherited methods
    fn build_vtable_with_inheritance(&self, class: &HirClass) -> Vec<String> {
        let mut vtable = Vec::new();
        
        // Start with parent's vtable (default to Object if no explicit parent)
        let parent_name = class.parent.clone().unwrap_or_else(|| "Object".to_string());
        if let Some(parent_vtable) = self.class_vtables.get(&parent_name) {
            vtable = parent_vtable.clone();
        }
        
        // Sort methods alphabetically to match class_hierarchy.get_all_methods()
        // This ensures vtable indices are consistent between ast_to_hir and lower
        let mut sorted_methods: Vec<_> = class.methods.iter().collect();
        sorted_methods.sort_by_key(|m| &m.name);
        
        // For each method in this class (alphabetically), either override or add
        for method in sorted_methods {
            let func_name = format!("{}_{}", class.name, method.name);
            
            // Check if this method overrides a parent method
            let mut found = false;
            for (i, existing) in vtable.iter().enumerate() {
                // Extract method name from "ClassName_methodName"
                if let Some(existing_method_name) = existing.split('_').last() {
                    if existing_method_name == method.name {
                        // Override parent's method
                        vtable[i] = func_name.clone();
                        found = true;
                        break;
                    }
                }
            }
            
            if !found {
                // New method - add to vtable
                vtable.push(func_name);
            }
        }
        
        vtable
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
                
                // For String comparison, use content comparison
                if *left.get_type() == TypeId::String {
                    // Call String_equals runtime function
                    instrs.push(LirInstr::Call("String_equals".to_string()));
                } else {
                    instrs.push(LirInstr::I32Eq);
                }
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
                    
                    // Store vtable pointer at offset 8
                    instrs.push(LirInstr::LocalGet(obj_local));
                    instrs.push(LirInstr::GetVTableAddr(type_name.clone()));
                    instrs.push(LirInstr::I32Store { offset: 8, align: 4 });
                    
                    // Save current "self" binding and set up new object as "self"
                    // This allows initializers to reference "self"
                    let old_self = self.locals.get("self").copied();
                    self.locals.insert("self".to_string(), obj_local);
                    
                    // Also set up attribute offsets for this class so initializers
                    // can access other attributes
                    let old_attribute_offsets = self.attribute_offsets.clone();
                    self.attribute_offsets.clear();
                    for (attr_name, _attr_type, attr_offset, _init) in &metadata.attributes {
                        self.attribute_offsets.insert(attr_name.clone(), *attr_offset);
                    }
                    
                    // Initialize attributes - evaluate initializers if present
                    for (attr_name, attr_type, attr_offset, attr_init) in &metadata.attributes {
                        instrs.push(LirInstr::comment(format!("init attr {}: {:?}", attr_name, attr_type)));
                        instrs.push(LirInstr::LocalGet(obj_local));
                        
                        if let Some(init_expr) = attr_init {
                            // Evaluate the initializer expression
                            instrs.extend(self.lower_expr(init_expr));
                        } else {
                            // Default value based on type
                            let default_value = match attr_type {
                                TypeId::Int => 0,      // Int defaults to 0
                                TypeId::Bool => 0,     // Bool defaults to false
                                TypeId::String => 0,   // String defaults to null (empty string ptr)
                                _ => 0,                // Reference types default to null (0)
                            };
                            instrs.push(LirInstr::I32Const(default_value));
                        }
                        instrs.push(LirInstr::I32Store { offset: *attr_offset, align: 4 });
                    }
                    
                    // Restore old "self" binding and attribute offsets
                    if let Some(old) = old_self {
                        self.locals.insert("self".to_string(), old);
                    } else {
                        self.locals.remove("self");
                    }
                    self.attribute_offsets = old_attribute_offsets;
                    
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
                
                // Dynamic dispatch via vtable
                instrs.push(LirInstr::comment(format!(
                    "Dynamic dispatch {}.{} (slot {})",
                    dispatch_info.class_name,
                    dispatch_info.method_name,
                    dispatch_info.method_index
                )));
                
                // Evaluate object (receiver) and save to local for later
                instrs.extend(self.lower_expr(object));
                let obj_local = self.alloc_local(LirType::I32);
                instrs.push(LirInstr::LocalSet(obj_local));
                
                // Now build the call: push receiver, push args, lookup func, call_indirect
                // First push the receiver
                instrs.push(LirInstr::LocalGet(obj_local));
                
                // Evaluate arguments - push them after receiver
                for arg in args {
                    instrs.extend(self.lower_expr(arg));
                }
                
                // Load vtable pointer from object (use local, don't consume receiver on stack)
                instrs.push(LirInstr::LocalGet(obj_local));
                instrs.push(LirInstr::I32Load { offset: 8, align: 4 });
                
                // Load function index from vtable at method_index * 4
                let vtable_offset = (dispatch_info.method_index * 4) as u32;
                instrs.push(LirInstr::I32Load { offset: vtable_offset, align: 4 });
                
                // Indirect call through the function table
                // type_index is the number of args + 1 (for self) with i32 result
                let num_params = args.len() + 1; // self + args
                instrs.push(LirInstr::CallIndirect { 
                    type_index: num_params as u32,
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
