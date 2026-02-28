/// AST to HIR lowering: Transforms parser AST into typed HIR
use parser::ast::{self, Program, Class, Feature, Expr};
use semant::{SemanticAnalyzer, class_hierarchy::ClassHierarchy};
use crate::hir::*;
use std::collections::HashMap;

/// Lower a program from AST to HIR
pub fn lower_program(
    program: &Program,
    analyzer: &SemanticAnalyzer,
) -> Result<HirProgram, String> {
    let mut lowerer = Lowerer::new(analyzer);
    lowerer.lower_program(program)
}

struct Lowerer<'a> {
    class_hierarchy: &'a ClassHierarchy,
    class_tags: HashMap<String, usize>,
    vtables: HashMap<String, Vec<String>>,
    /// Maps (class_name, method_name) -> defining_class_name
    method_owners: HashMap<(String, String), String>,
    current_class: Option<String>,
}

impl<'a> Lowerer<'a> {
    fn new(analyzer: &'a SemanticAnalyzer) -> Self {
        Lowerer {
            class_hierarchy: analyzer.class_hierarchy(),
            class_tags: HashMap::new(),
            vtables: HashMap::new(),
            method_owners: HashMap::new(),
            current_class: None,
        }
    }

    fn lower_program(&mut self, program: &Program) -> Result<HirProgram, String> {
        // Assign class tags (unique integers for runtime type checks)
        let mut next_tag = 0;
        for class in &program.classes {
            self.class_tags.insert(class.name.clone(), next_tag);
            next_tag += 1;
        }

        // Build vtables and method ownership tracking for all classes
        for class in &program.classes {
            self.build_vtable_and_owners(&class.name, class);
        }

        // Lower each class
        let mut hir_classes = Vec::new();
        for class in &program.classes {
            let hir_class = self.lower_class(class)?;
            hir_classes.push(hir_class);
        }

        Ok(HirProgram {
            classes: hir_classes,
        })
    }

    fn build_vtable_and_owners(&mut self, class_name: &str, class: &Class) {
        let mut vtable = Vec::new();
        
        // Get all methods (including inherited) in order
        let methods = self.class_hierarchy.get_all_methods(class_name);
        for (method_name, _sig) in methods {
            vtable.push(method_name.clone());
            
            // Determine which class defines this method
            let defining_class = self.find_method_owner(class_name, &method_name, class);
            self.method_owners.insert(
                (class_name.to_string(), method_name),
                defining_class
            );
        }

        self.vtables.insert(class_name.to_string(), vtable);
    }
    
    fn find_method_owner(&self, class_name: &str, method_name: &str, class: &Class) -> String {
        // Check if this class defines the method
        for feature in &class.features {
            if let Feature::Method(m) = feature {
                if m.name == method_name {
                    return class_name.to_string();
                }
            }
        }
        
        // Walk up inheritance chain to find which class defines it
        let mut current = class.parent.clone();
        while let Some(parent_name) = current {
            // Use class_hierarchy to check if this parent defines the method
            if let Some(parent_info) = self.class_hierarchy.get_class(&parent_name) {
                if parent_info.methods.contains_key(method_name) {
                    return parent_name.clone();
                }
            }
            
            // Try next parent
            current = self.class_hierarchy.get_parent(&parent_name);
        }
        
        // Default to current class if we can't find it
        class_name.to_string()
    }

    fn get_vtable_index(&self, class_name: &str, method_name: &str) -> usize {
        self.vtables
            .get(class_name)
            .and_then(|vtable| vtable.iter().position(|m| m == method_name))
            .unwrap_or(0)
    }

    fn lower_class(&mut self, class: &Class) -> Result<HirClass, String> {
        self.current_class = Some(class.name.clone());

        let class_tag = *self.class_tags.get(&class.name).unwrap_or(&0);

        // Lower attributes
        let mut hir_attributes = Vec::new();
        let mut attr_offset = 0;
        for feature in &class.features {
            if let Feature::Attribute(attr) = feature {
                let typ = TypeId::from_string(&attr.attr_type, Some(&class.name));
                let init = if let Some(ref init_expr) = attr.init {
                    Some(self.lower_expr(init_expr)?)
                } else {
                    None
                };

                hir_attributes.push(HirAttribute {
                    name: attr.name.clone(),
                    typ,
                    init,
                    offset: attr_offset,
                });
                attr_offset += 1; // Simple offset calculation
            }
        }

        // Lower methods
        let mut hir_methods = Vec::new();
        for feature in &class.features {
            if let Feature::Method(method) = feature {
                let vtable_index = self.get_vtable_index(&class.name, &method.name);
                let hir_method = self.lower_method(method, vtable_index)?;
                hir_methods.push(hir_method);
            }
        }

        self.current_class = None;

        Ok(HirClass {
            name: class.name.clone(),
            parent: class.parent.clone(),
            class_tag,
            attributes: hir_attributes,
            methods: hir_methods,
        })
    }

    fn lower_method(&mut self, method: &ast::MethodFeature, vtable_index: usize) -> Result<HirMethod, String> {
        let formals = method
            .formals
            .iter()
            .map(|f| HirFormal {
                name: f.name.clone(),
                typ: TypeId::from_string(&f.typ, self.current_class.as_deref()),
            })
            .collect();

        let return_type = TypeId::from_string(&method.return_type, self.current_class.as_deref());
        let body = self.lower_expr(&method.body)?;

        Ok(HirMethod {
            name: method.name.clone(),
            formals,
            return_type,
            body,
            vtable_index,
        })
    }

    fn lower_expr(&mut self, expr: &Expr) -> Result<HirExpr, String> {
        match expr {
            // Literals
            Expr::Integer(value) => Ok(HirExpr::IntLiteral {
                value: *value,
                typ: TypeId::Int,
            }),
            
            Expr::String(value) => Ok(HirExpr::StringLiteral {
                value: value.clone(),
                typ: TypeId::String,
            }),
            
            Expr::True => Ok(HirExpr::BoolLiteral {
                value: true,
                typ: TypeId::Bool,
            }),
            
            Expr::False => Ok(HirExpr::BoolLiteral {
                value: false,
                typ: TypeId::Bool,
            }),

            // Variables
            Expr::Id(name) => {
                let typ = if name == "self" {
                    if let Some(ref class) = self.current_class {
                        TypeId::SelfType(class.clone())
                    } else {
                        TypeId::NoType
                    }
                } else {
                    // In production, we'd look this up in the symbol table
                    // For now, use NoType as placeholder
                    TypeId::NoType
                };
                
                Ok(HirExpr::Object {
                    name: name.clone(),
                    typ,
                })
            },

            // Assignment
            Expr::Assign { name, expr: value } => {
                let expr_hir = self.lower_expr(value)?;
                let typ = expr_hir.get_type().clone();
                
                Ok(HirExpr::Assign {
                    name: name.clone(),
                    expr: Box::new(expr_hir),
                    typ,
                })
            },

            // Arithmetic operations
            Expr::Plus(left, right) => {
                Ok(HirExpr::Add {
                    left: Box::new(self.lower_expr(left)?),
                    right: Box::new(self.lower_expr(right)?),
                    typ: TypeId::Int,
                })
            },

            Expr::Minus(left, right) => {
                Ok(HirExpr::Sub {
                    left: Box::new(self.lower_expr(left)?),
                    right: Box::new(self.lower_expr(right)?),
                    typ: TypeId::Int,
                })
            },

            Expr::Times(left, right) => {
                Ok(HirExpr::Mul {
                    left: Box::new(self.lower_expr(left)?),
                    right: Box::new(self.lower_expr(right)?),
                    typ: TypeId::Int,
                })
            },

            Expr::Divide(left, right) => {
                Ok(HirExpr::Div {
                    left: Box::new(self.lower_expr(left)?),
                    right: Box::new(self.lower_expr(right)?),
                    typ: TypeId::Int,
                })
            },

            // Comparison operations
            Expr::Lt(left, right) => {
                Ok(HirExpr::Lt {
                    left: Box::new(self.lower_expr(left)?),
                    right: Box::new(self.lower_expr(right)?),
                    typ: TypeId::Bool,
                })
            },

            Expr::Le(left, right) => {
                Ok(HirExpr::Le {
                    left: Box::new(self.lower_expr(left)?),
                    right: Box::new(self.lower_expr(right)?),
                    typ: TypeId::Bool,
                })
            },

            Expr::Eq(left, right) => {
                Ok(HirExpr::Eq {
                    left: Box::new(self.lower_expr(left)?),
                    right: Box::new(self.lower_expr(right)?),
                    typ: TypeId::Bool,
                })
            },

            // Unary operations
            Expr::Not(expr) => {
                Ok(HirExpr::Not {
                    expr: Box::new(self.lower_expr(expr)?),
                    typ: TypeId::Bool,
                })
            },

            Expr::Negate(expr) => {
                Ok(HirExpr::Negate {
                    expr: Box::new(self.lower_expr(expr)?),
                    typ: TypeId::Int,
                })
            },

            Expr::IsVoid(expr) => {
                Ok(HirExpr::IsVoid {
                    expr: Box::new(self.lower_expr(expr)?),
                    typ: TypeId::Bool,
                })
            },

            // Object creation
            Expr::New(type_name) => {
                let typ = TypeId::from_string(type_name, self.current_class.as_deref());
                Ok(HirExpr::New {
                    type_name: type_name.clone(),
                    typ,
                })
            },

            // Control flow
            Expr::If { cond, then_branch, else_branch } => {
                let cond_hir = self.lower_expr(cond)?;
                let then_hir = self.lower_expr(then_branch)?;
                let else_hir = self.lower_expr(else_branch)?;
                
                // Type is LUB of then/else branches (simplified: use then branch type)
                let typ = then_hir.get_type().clone();
                
                Ok(HirExpr::If {
                    cond: Box::new(cond_hir),
                    then_branch: Box::new(then_hir),
                    else_branch: Box::new(else_hir),
                    typ,
                })
            },

            Expr::While { cond, body } => {
                Ok(HirExpr::While {
                    cond: Box::new(self.lower_expr(cond)?),
                    body: Box::new(self.lower_expr(body)?),
                    typ: TypeId::Object, // While always returns Object
                })
            },

            Expr::Block(exprs) => {
                let mut hir_exprs = Vec::new();
                for expr in exprs {
                    hir_exprs.push(self.lower_expr(expr)?);
                }
                
                let typ = if let Some(last) = hir_exprs.last() {
                    last.get_type().clone()
                } else {
                    TypeId::NoType
                };
                
                Ok(HirExpr::Block {
                    exprs: hir_exprs,
                    typ,
                })
            },

            // Let binding
            Expr::Let { bindings, body } => {
                // Handle nested let bindings by converting to nested Let expressions
                if bindings.is_empty() {
                    return self.lower_expr(body);
                }

                // Process bindings from left to right, creating nested Let expressions
                let mut result = self.lower_expr(body)?;
                let result_type = result.get_type().clone();
                
                // Work backwards through bindings to create nested structure
                for binding in bindings.iter().rev() {
                    let current_class_copy = self.current_class.clone();
                    let typ = TypeId::from_string(&binding.typ, current_class_copy.as_deref());
                    let init = if let Some(ref init_expr) = binding.init {
                        Box::new(self.lower_expr(init_expr)?)
                    } else {
                        // Default initialization based on type
                        Box::new(match typ {
                            TypeId::Int => HirExpr::IntLiteral { value: 0, typ: TypeId::Int },
                            TypeId::Bool => HirExpr::BoolLiteral { value: false, typ: TypeId::Bool },
                            TypeId::String => HirExpr::StringLiteral { value: String::new(), typ: TypeId::String },
                            _ => HirExpr::Object { name: "void".to_string(), typ: TypeId::NoType },
                        })
                    };

                    result = HirExpr::Let {
                        name: binding.name.clone(),
                        typ,
                        init,
                        body: Box::new(result),
                        result_type: result_type.clone(),
                    };
                }

                Ok(result)
            },

            // Case expression
            Expr::Case { expr, branches } => {
                let expr_hir = self.lower_expr(expr)?;
                let mut hir_branches = Vec::new();
                
                for branch in branches {
                    let branch_expr = self.lower_expr(&branch.expr)?;
                    hir_branches.push(CaseBranch {
                        name: branch.name.clone(),
                        type_name: branch.typ.clone(),
                        expr: branch_expr,
                    });
                }
                
                // Type is LUB of all branch types (simplified: use first branch)
                let typ = if let Some(first) = hir_branches.first() {
                    first.expr.get_type().clone()
                } else {
                    TypeId::NoType
                };
                
                Ok(HirExpr::Case {
                    expr: Box::new(expr_hir),
                    branches: hir_branches,
                    typ,
                })
            },

            // Method dispatch
            Expr::Dispatch { expr, static_type, method, args } => {
                let object = Box::new(self.lower_expr(expr)?);
                let object_type = object.get_type().clone();
                
                let mut hir_args = Vec::new();
                for arg in args {
                    hir_args.push(self.lower_expr(arg)?);
                }

                if let Some(ref static_class) = static_type {
                    // Static dispatch: expr@Type.method(args)
                    let method_index = self.get_vtable_index(static_class, method);
                    
                    // Find which class actually defines this method
                    let defining_class = self.method_owners
                        .get(&(static_class.clone(), method.clone()))
                        .cloned()
                        .unwrap_or_else(|| static_class.clone());
                    
                    Ok(HirExpr::StaticDispatch {
                        object,
                        type_name: static_class.clone(),
                        dispatch_info: DispatchInfo {
                            class_name: defining_class,
                            method_name: method.clone(),
                            method_index,
                        },
                        args: hir_args,
                        typ: TypeId::NoType, // Would be resolved by type checker
                    })
                } else {
                    // Dynamic dispatch: expr.method(args)
                    let class_name = match &object_type {
                        TypeId::Class(name) => name.clone(),
                        TypeId::SelfType(name) => name.clone(),
                        _ => "Object".to_string(),
                    };
                    
                    let method_index = self.get_vtable_index(&class_name, method);
                    
                    // Find which class actually defines this method
                    let defining_class = self.method_owners
                        .get(&(class_name.clone(), method.clone()))
                        .cloned()
                        .unwrap_or(class_name.clone());
                    
                    Ok(HirExpr::Dispatch {
                        object,
                        dispatch_info: DispatchInfo {
                            class_name: defining_class,
                            method_name: method.clone(),
                            method_index,
                        },
                        args: hir_args,
                        typ: TypeId::NoType, // Would be resolved by type checker
                    })
                }
            },

            // Function call (self method call)
            Expr::FuncCall { name, args } => {
                // Convert to dispatch on self
                let current_class_copy = self.current_class.clone();
                let self_expr = Box::new(HirExpr::Object {
                    name: "self".to_string(),
                    typ: if let Some(ref class) = current_class_copy {
                        TypeId::SelfType(class.clone())
                    } else {
                        TypeId::NoType
                    },
                });

                let mut hir_args = Vec::new();
                for arg in args {
                    hir_args.push(self.lower_expr(arg)?);
                }

                let class_name = current_class_copy.clone().unwrap_or_else(|| "Object".to_string());
                let method_index = self.get_vtable_index(&class_name, name);
                
                // Find which class actually defines this method
                let defining_class = self.method_owners
                    .get(&(class_name.clone(), name.clone()))
                    .cloned()
                    .unwrap_or(class_name.clone());

                Ok(HirExpr::Dispatch {
                    object: self_expr,
                    dispatch_info: DispatchInfo {
                        class_name: defining_class,
                        method_name: name.clone(),
                        method_index,
                    },
                    args: hir_args,
                    typ: TypeId::NoType,
                })
            },

            Expr::Paren(expr) => {
                // Parentheses don't affect HIR
                self.lower_expr(expr)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::ast::{Program, Class, Feature, MethodFeature, Expr};

    #[test]
    fn test_lower_simple_class() {
        let program = Program {
            classes: vec![Class {
                name: "Main".to_string(),
                parent: None,
                features: vec![Feature::Method(MethodFeature {
                    name: "main".to_string(),
                    formals: vec![],
                    return_type: "Object".to_string(),
                    body: Expr::Integer(42),
                })],
            }],
        };

        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze(&program);
        
        let result = lower_program(&program, &analyzer);
        assert!(result.is_ok());
        
        let hir = result.unwrap();
        assert_eq!(hir.classes.len(), 1);
        assert_eq!(hir.classes[0].name, "Main");
        assert_eq!(hir.classes[0].methods.len(), 1);
    }

    #[test]
    fn test_lower_arithmetic() {
        let expr = Expr::Plus(
            Box::new(Expr::Integer(1)),
            Box::new(Expr::Integer(2)),
        );

        let analyzer = SemanticAnalyzer::new();
        let mut lowerer = Lowerer::new(&analyzer);
        
        let result = lowerer.lower_expr(&expr);
        assert!(result.is_ok());
        
        match result.unwrap() {
            HirExpr::Add { left, right, typ } => {
                assert!(matches!(*left, HirExpr::IntLiteral { value: 1, .. }));
                assert!(matches!(*right, HirExpr::IntLiteral { value: 2, .. }));
                assert_eq!(typ, TypeId::Int);
            }
            _ => panic!("Expected Add expression"),
        }
    }

    #[test]
    fn test_lower_control_flow() {
        let expr = Expr::If {
            cond: Box::new(Expr::True),
            then_branch: Box::new(Expr::Integer(1)),
            else_branch: Box::new(Expr::Integer(2)),
        };

        let analyzer = SemanticAnalyzer::new();
        let mut lowerer = Lowerer::new(&analyzer);
        
        let result = lowerer.lower_expr(&expr);
        assert!(result.is_ok());
        
        match result.unwrap() {
            HirExpr::If { cond, then_branch, else_branch, .. } => {
                assert!(matches!(*cond, HirExpr::BoolLiteral { value: true, .. }));
                assert!(matches!(*then_branch, HirExpr::IntLiteral { value: 1, .. }));
                assert!(matches!(*else_branch, HirExpr::IntLiteral { value: 2, .. }));
            }
            _ => panic!("Expected If expression"),
        }
    }
}
