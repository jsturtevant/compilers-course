use crate::{ClassHierarchy, SemanticError, SymbolTable};
use parser::ast::{Expr, Program};

/// Type represents a COOL type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Class(String),
    SelfType,
    NoType, // For error recovery
}

impl Type {
    pub fn from_string(s: &str) -> Self {
        match s {
            "SELF_TYPE" => Type::SelfType,
            _ => Type::Class(s.to_string()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Type::Class(name) => name.clone(),
            Type::SelfType => "SELF_TYPE".to_string(),
            Type::NoType => "_no_type".to_string(),
        }
    }
}

/// TypeChecker performs type inference and checking on expressions
pub struct TypeChecker {
    current_class: Option<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            current_class: None,
        }
    }

    pub fn set_current_class(&mut self, class_name: String) {
        self.current_class = Some(class_name);
    }

    /// Type check a program
    pub fn check_program(
        &mut self,
        _program: &Program,
        _hierarchy: &ClassHierarchy,
        _symbol_table: &mut SymbolTable,
    ) -> Result<(), Vec<SemanticError>> {
        // Implementation delegated to Hopper
        // Should iterate through all classes and type check their features
        Ok(())
    }

    /// Infer the type of an expression
    pub fn infer_type(
        &self,
        expr: &Expr,
        hierarchy: &ClassHierarchy,
        symbol_table: &mut SymbolTable,
    ) -> Result<Type, SemanticError> {
        match expr {
            Expr::Integer(_) => Ok(Type::Class("Int".to_string())),
            Expr::String(_) => Ok(Type::Class("String".to_string())),
            Expr::True | Expr::False => Ok(Type::Class("Bool".to_string())),

            Expr::Id(name) => {
                if name == "self" {
                    Ok(Type::SelfType)
                } else {
                    symbol_table
                        .lookup_variable(name)
                        .map(|t| Type::from_string(t))
                        .ok_or_else(|| SemanticError::UndefinedVariable {
                            name: name.clone(),
                            line: 0,
                        })
                }
            }

            Expr::New(class_name) => {
                if class_name == "SELF_TYPE" {
                    Ok(Type::SelfType)
                } else if hierarchy.class_exists(class_name) {
                    Ok(Type::Class(class_name.clone()))
                } else {
                    Err(SemanticError::UndefinedType {
                        name: class_name.clone(),
                        line: 0,
                    })
                }
            }

            Expr::Plus(left, right)
            | Expr::Minus(left, right)
            | Expr::Times(left, right)
            | Expr::Divide(left, right) => {
                let left_type = self.infer_type(left, hierarchy, symbol_table)?;
                let right_type = self.infer_type(right, hierarchy, symbol_table)?;

                if !matches!(left_type, Type::Class(ref name) if name == "Int") {
                    return Err(SemanticError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: left_type.to_string(),
                        line: 0,
                    });
                }

                if !matches!(right_type, Type::Class(ref name) if name == "Int") {
                    return Err(SemanticError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: right_type.to_string(),
                        line: 0,
                    });
                }

                Ok(Type::Class("Int".to_string()))
            }

            Expr::Lt(left, right) | Expr::Le(left, right) => {
                let left_type = self.infer_type(left, hierarchy, symbol_table)?;
                let right_type = self.infer_type(right, hierarchy, symbol_table)?;

                if !matches!(left_type, Type::Class(ref name) if name == "Int") {
                    return Err(SemanticError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: left_type.to_string(),
                        line: 0,
                    });
                }

                if !matches!(right_type, Type::Class(ref name) if name == "Int") {
                    return Err(SemanticError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: right_type.to_string(),
                        line: 0,
                    });
                }

                Ok(Type::Class("Bool".to_string()))
            }

            Expr::Not(expr) => {
                let expr_type = self.infer_type(expr, hierarchy, symbol_table)?;
                if !matches!(expr_type, Type::Class(ref name) if name == "Bool") {
                    return Err(SemanticError::TypeMismatch {
                        expected: "Bool".to_string(),
                        found: expr_type.to_string(),
                        line: 0,
                    });
                }
                Ok(Type::Class("Bool".to_string()))
            }

            Expr::Negate(expr) => {
                let expr_type = self.infer_type(expr, hierarchy, symbol_table)?;
                if !matches!(expr_type, Type::Class(ref name) if name == "Int") {
                    return Err(SemanticError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: expr_type.to_string(),
                        line: 0,
                    });
                }
                Ok(Type::Class("Int".to_string()))
            }

            Expr::IsVoid(_) => Ok(Type::Class("Bool".to_string())),

            Expr::Paren(expr) => self.infer_type(expr, hierarchy, symbol_table),

            Expr::Eq(left, right) => {
                let left_type = self.infer_type(left, hierarchy, symbol_table)?;
                let right_type = self.infer_type(right, hierarchy, symbol_table)?;

                // Equality is defined for Int, String, and Bool
                let basic_types = ["Int", "String", "Bool"];
                let left_is_basic =
                    matches!(&left_type, Type::Class(name) if basic_types.contains(&name.as_str()));
                let right_is_basic = matches!(&right_type, Type::Class(name) if basic_types.contains(&name.as_str()));

                if left_is_basic && right_is_basic {
                    if left_type != right_type {
                        return Err(SemanticError::TypeMismatch {
                            expected: left_type.to_string(),
                            found: right_type.to_string(),
                            line: 0,
                        });
                    }
                }

                Ok(Type::Class("Bool".to_string()))
            }

            Expr::Assign { name, expr } => {
                let expr_type = self.infer_type(expr, hierarchy, symbol_table)?;

                if let Some(var_type_str) = symbol_table.lookup_variable(name) {
                    let var_type = Type::from_string(var_type_str);
                    if !self.check_conformance(&expr_type, &var_type, hierarchy) {
                        return Err(SemanticError::TypeMismatch {
                            expected: var_type.to_string(),
                            found: expr_type.to_string(),
                            line: 0,
                        });
                    }
                    Ok(expr_type)
                } else {
                    Err(SemanticError::UndefinedVariable {
                        name: name.clone(),
                        line: 0,
                    })
                }
            }

            Expr::Block(exprs) => {
                if exprs.is_empty() {
                    return Ok(Type::NoType);
                }

                let mut last_type = Type::NoType;
                for expr in exprs {
                    last_type = self.infer_type(expr, hierarchy, symbol_table)?;
                }
                Ok(last_type)
            }

            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let _cond_type = self.infer_type(cond, hierarchy, symbol_table)?;
                // In COOL, conditions just need to be expressions - they don't have to be Bool
                // The runtime will treat 0 as false and non-zero as true
                // So we accept any type here and don't check

                let then_type = self.infer_type(then_branch, hierarchy, symbol_table)?;
                let else_type = self.infer_type(else_branch, hierarchy, symbol_table)?;

                // Join the types
                let result_type = match (&then_type, &else_type) {
                    (Type::Class(t1), Type::Class(t2)) => {
                        Type::Class(hierarchy.least_upper_bound(t1, t2))
                    }
                    _ => then_type,
                };

                Ok(result_type)
            }

            Expr::While { cond, body } => {
                let _cond_type = self.infer_type(cond, hierarchy, symbol_table)?;
                // In COOL, conditions can be any type - runtime treats 0 as false, non-zero as true
                // So we don't check the condition type

                self.infer_type(body, hierarchy, symbol_table)?;
                Ok(Type::Class("Object".to_string()))
            }

            Expr::Let { bindings, body } => {
                // Create a new scope for let bindings
                symbol_table.push_scope();

                // Process each binding and add to symbol table progressively
                // so that later bindings can reference earlier ones
                for binding in bindings {
                    let init_type = if let Some(ref init_expr) = binding.init {
                        self.infer_type(init_expr, hierarchy, symbol_table)?
                    } else {
                        Type::from_string(&binding.typ)
                    };

                    let declared_type = Type::from_string(&binding.typ);
                    if !self.check_conformance(&init_type, &declared_type, hierarchy) {
                        symbol_table.pop_scope();
                        return Err(SemanticError::TypeMismatch {
                            expected: declared_type.to_string(),
                            found: init_type.to_string(),
                            line: 0,
                        });
                    }

                    // Add variable to scope after checking init expression
                    if let Err(_) =
                        symbol_table.add_variable(binding.name.clone(), binding.typ.clone())
                    {
                        symbol_table.pop_scope();
                        return Err(SemanticError::RedefinedAttribute {
                            class: self.current_class.clone().unwrap_or_default(),
                            attr: binding.name.clone(),
                            line: 0,
                        });
                    }
                }

                let result = self.infer_type(body, hierarchy, symbol_table);
                symbol_table.pop_scope();
                result
            }

            Expr::Case { expr, branches } => {
                self.infer_type(expr, hierarchy, symbol_table)?;

                if branches.is_empty() {
                    return Ok(Type::NoType);
                }

                // Type of case is LUB of all branch types
                let mut result_type = Type::NoType;
                for (i, branch) in branches.iter().enumerate() {
                    // Each case branch introduces a new scope with the branch variable
                    symbol_table.push_scope();

                    // Add the case variable to scope
                    if let Err(_) =
                        symbol_table.add_variable(branch.name.clone(), branch.typ.clone())
                    {
                        symbol_table.pop_scope();
                        return Err(SemanticError::RedefinedAttribute {
                            class: self.current_class.clone().unwrap_or_default(),
                            attr: branch.name.clone(),
                            line: 0,
                        });
                    }

                    let branch_type = self.infer_type(&branch.expr, hierarchy, symbol_table)?;
                    symbol_table.pop_scope();

                    if i == 0 {
                        result_type = branch_type;
                    } else {
                        result_type = match (&result_type, &branch_type) {
                            (Type::Class(t1), Type::Class(t2)) => {
                                Type::Class(hierarchy.least_upper_bound(t1, t2))
                            }
                            _ => result_type,
                        };
                    }
                }

                Ok(result_type)
            }

            Expr::Dispatch {
                expr,
                static_type,
                method,
                args,
            } => {
                let obj_type = self.infer_type(expr, hierarchy, symbol_table)?;

                let mut class_name = match static_type {
                    Some(ref type_name) => type_name.clone(),
                    None => obj_type.to_string(),
                };

                // Resolve SELF_TYPE to current class for method lookup
                if class_name == "SELF_TYPE" {
                    if let Some(ref current) = self.current_class {
                        class_name = current.clone();
                    }
                }

                if let Some(method_info) = hierarchy.get_method(&class_name, method) {
                    // Check argument count
                    if args.len() != method_info.param_types.len() {
                        return Err(SemanticError::TypeMismatch {
                            expected: format!("{} arguments", method_info.param_types.len()),
                            found: format!("{} arguments", args.len()),
                            line: 0,
                        });
                    }

                    // Check argument types
                    for (arg, param_type) in args.iter().zip(method_info.param_types.iter()) {
                        let arg_type = self.infer_type(arg, hierarchy, symbol_table)?;
                        let expected_type = Type::from_string(param_type);
                        if !self.check_conformance(&arg_type, &expected_type, hierarchy) {
                            return Err(SemanticError::TypeMismatch {
                                expected: expected_type.to_string(),
                                found: arg_type.to_string(),
                                line: 0,
                            });
                        }
                    }

                    // Return type, handling SELF_TYPE
                    if method_info.return_type == "SELF_TYPE" {
                        Ok(obj_type)
                    } else {
                        Ok(Type::from_string(&method_info.return_type))
                    }
                } else {
                    Err(SemanticError::UndefinedMethod {
                        class: class_name,
                        method: method.clone(),
                        line: 0,
                    })
                }
            }

            Expr::FuncCall { name, args } => {
                // Function call is dispatch on self
                if let Some(ref current_class) = self.current_class {
                    if let Some(method_info) = hierarchy.get_method(current_class, name) {
                        // Check arguments
                        if args.len() != method_info.param_types.len() {
                            return Err(SemanticError::TypeMismatch {
                                expected: format!("{} arguments", method_info.param_types.len()),
                                found: format!("{} arguments", args.len()),
                                line: 0,
                            });
                        }

                        for (arg, param_type) in args.iter().zip(method_info.param_types.iter()) {
                            let arg_type = self.infer_type(arg, hierarchy, symbol_table)?;
                            let expected_type = Type::from_string(param_type);
                            if !self.check_conformance(&arg_type, &expected_type, hierarchy) {
                                return Err(SemanticError::TypeMismatch {
                                    expected: expected_type.to_string(),
                                    found: arg_type.to_string(),
                                    line: 0,
                                });
                            }
                        }

                        if method_info.return_type == "SELF_TYPE" {
                            Ok(Type::SelfType)
                        } else {
                            Ok(Type::from_string(&method_info.return_type))
                        }
                    } else {
                        Err(SemanticError::UndefinedMethod {
                            class: current_class.clone(),
                            method: name.clone(),
                            line: 0,
                        })
                    }
                } else {
                    Err(SemanticError::UndefinedMethod {
                        class: "unknown".to_string(),
                        method: name.clone(),
                        line: 0,
                    })
                }
            }
        }
    }

    /// Check if type1 conforms to type2
    pub fn check_conformance(
        &self,
        type1: &Type,
        type2: &Type,
        hierarchy: &ClassHierarchy,
    ) -> bool {
        match (type1, type2) {
            (Type::NoType, _) | (_, Type::NoType) => false,
            (Type::SelfType, Type::SelfType) => true,
            (Type::SelfType, Type::Class(_)) => {
                // SELF_TYPE conforms to current class
                if let Some(ref current) = self.current_class {
                    if let Type::Class(target) = type2 {
                        hierarchy.conforms(current, target)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            (Type::Class(_), Type::SelfType) => false,
            (Type::Class(c1), Type::Class(c2)) => hierarchy.conforms(c1, c2),
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_from_string() {
        assert_eq!(Type::from_string("Int"), Type::Class("Int".to_string()));
        assert_eq!(Type::from_string("SELF_TYPE"), Type::SelfType);
    }

    #[test]
    fn test_type_to_string() {
        assert_eq!(Type::Class("Int".to_string()).to_string(), "Int");
        assert_eq!(Type::SelfType.to_string(), "SELF_TYPE");
    }

    #[test]
    fn test_literal_types() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        assert_eq!(
            checker
                .infer_type(&Expr::Integer(42), &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Int".to_string())
        );

        assert_eq!(
            checker
                .infer_type(
                    &Expr::String("hello".to_string()),
                    &hierarchy,
                    &mut symbol_table
                )
                .unwrap(),
            Type::Class("String".to_string())
        );

        assert_eq!(
            checker
                .infer_type(&Expr::True, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Bool".to_string())
        );
    }

    #[test]
    fn test_arithmetic_operations() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::Plus(Box::new(Expr::Integer(1)), Box::new(Expr::Integer(2)));

        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Int".to_string())
        );
    }

    #[test]
    fn test_comparison_operations() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::Lt(Box::new(Expr::Integer(1)), Box::new(Expr::Integer(2)));

        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Bool".to_string())
        );
    }

    #[test]
    fn test_not_operation() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::Not(Box::new(Expr::True));

        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Bool".to_string())
        );
    }

    #[test]
    fn test_isvoid_operation() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::IsVoid(Box::new(Expr::Integer(42)));

        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Bool".to_string())
        );
    }

    #[test]
    fn test_block_expression() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::Block(vec![
            Expr::Integer(1),
            Expr::String("hello".to_string()),
            Expr::True,
        ]);

        // Block type is the type of the last expression
        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Bool".to_string())
        );
    }

    #[test]
    fn test_if_expression() {
        let checker = TypeChecker::new();
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::If {
            cond: Box::new(Expr::True),
            then_branch: Box::new(Expr::Integer(1)),
            else_branch: Box::new(Expr::Integer(2)),
        };

        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Int".to_string())
        );
    }

    #[test]
    fn test_while_expression() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::While {
            cond: Box::new(Expr::True),
            body: Box::new(Expr::Integer(1)),
        };

        // While always returns Object
        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Object".to_string())
        );
    }

    #[test]
    fn test_new_expression() {
        let checker = TypeChecker::new();
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::New("Int".to_string());

        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::Class("Int".to_string())
        );
    }

    #[test]
    fn test_self_type() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::Id("self".to_string());

        assert_eq!(
            checker
                .infer_type(&expr, &hierarchy, &mut symbol_table)
                .unwrap(),
            Type::SelfType
        );
    }

    #[test]
    fn test_undefined_variable() {
        let checker = TypeChecker::new();
        let hierarchy = ClassHierarchy::new();
        let mut symbol_table = SymbolTable::new();

        let expr = Expr::Id("undefined_var".to_string());

        assert!(checker
            .infer_type(&expr, &hierarchy, &mut symbol_table)
            .is_err());
    }
}
