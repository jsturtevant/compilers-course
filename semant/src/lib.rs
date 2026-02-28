pub mod symbol_table;
pub mod class_hierarchy;
pub mod types;

pub use symbol_table::ScopeStack as SymbolTable;
pub use class_hierarchy::ClassHierarchy;
pub use types::{Type, TypeChecker};

use parser::ast::Program;

#[derive(Debug, Clone)]
pub enum SemanticError {
    DuplicateClass { name: String, line: usize },
    UndefinedType { name: String, line: usize },
    CyclicInheritance { path: Vec<String> },
    UndefinedVariable { name: String, line: usize },
    UndefinedMethod { class: String, method: String, line: usize },
    TypeMismatch { expected: String, found: String, line: usize },
    InvalidInheritance { class: String, parent: String, line: usize },
    RedefinedAttribute { class: String, attr: String, line: usize },
    InvalidOverride { class: String, method: String, reason: String, line: usize },
}

pub struct SemanticAnalyzer {
    class_hierarchy: ClassHierarchy,
    symbol_table: SymbolTable,
    type_checker: TypeChecker,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut class_hierarchy = ClassHierarchy::new();
        class_hierarchy.add_builtins();
        
        SemanticAnalyzer {
            class_hierarchy,
            symbol_table: SymbolTable::new(),
            type_checker: TypeChecker::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        // Phase 1: Build class hierarchy
        if let Err(e) = self.class_hierarchy.build(&program.classes) {
            errors.extend(e);
        }

        // Phase 2: Check for cycles
        if let Err(e) = self.class_hierarchy.check_cycles() {
            errors.extend(e);
        }

        // Phase 3: Validate class features and type check
        if errors.is_empty() {
            if let Err(e) = self.check_program(program) {
                errors.extend(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_program(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        // Iterate through all classes and type check their features
        for class in &program.classes {
            self.type_checker.set_current_class(class.name.clone());
            self.symbol_table.push_scope();
            self.symbol_table.add_self(&class.name);

            // Add all attributes (including inherited) to symbol table
            let all_attributes = self.class_hierarchy.get_all_attributes(&class.name);
            for (attr_name, attr_type) in &all_attributes {
                if let Err(_) = self.symbol_table.add_variable(attr_name.clone(), attr_type.clone()) {
                    errors.push(SemanticError::RedefinedAttribute {
                        class: class.name.clone(),
                        attr: attr_name.clone(),
                        line: 0,
                    });
                }
            }

            // Type check each feature
            for feature in &class.features {
                match feature {
                    parser::ast::Feature::Method(m) => {
                        self.symbol_table.push_scope();

                        // Add parameters to symbol table
                        for formal in &m.formals {
                            if let Err(_) = self.symbol_table.add_variable(formal.name.clone(), formal.typ.clone()) {
                                errors.push(SemanticError::RedefinedAttribute {
                                    class: class.name.clone(),
                                    attr: formal.name.clone(),
                                    line: 0,
                                });
                            }
                        }

                        // Type check method body
                        match self.type_checker.infer_type(&m.body, &self.class_hierarchy, &mut self.symbol_table) {
                            Ok(body_type) => {
                                let expected_type = types::Type::from_string(&m.return_type);
                                if !self.type_checker.check_conformance(&body_type, &expected_type, &self.class_hierarchy) {
                                    errors.push(SemanticError::TypeMismatch {
                                        expected: m.return_type.clone(),
                                        found: body_type.to_string(),
                                        line: 0,
                                    });
                                }
                            }
                            Err(e) => errors.push(e),
                        }

                        self.symbol_table.pop_scope();
                    }
                    parser::ast::Feature::Attribute(a) => {
                        // Type check attribute initializer if present
                        if let Some(ref init) = a.init {
                            match self.type_checker.infer_type(init, &self.class_hierarchy, &mut self.symbol_table) {
                                Ok(init_type) => {
                                    let expected_type = types::Type::from_string(&a.attr_type);
                                    if !self.type_checker.check_conformance(&init_type, &expected_type, &self.class_hierarchy) {
                                        errors.push(SemanticError::TypeMismatch {
                                            expected: a.attr_type.clone(),
                                            found: init_type.to_string(),
                                            line: 0,
                                        });
                                    }
                                }
                                Err(e) => errors.push(e),
                            }
                        }
                    }
                }
            }

            self.symbol_table.pop_scope();
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
