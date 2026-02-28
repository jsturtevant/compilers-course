use crate::SemanticError;
use parser::ast::Class;
use std::collections::{HashMap, HashSet};

/// ClassInfo stores information about a class in the hierarchy
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub parent: Option<String>,
    pub attributes: HashMap<String, String>,
    pub methods: HashMap<String, MethodSignature>,
    pub is_builtin: bool,
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub param_types: Vec<String>,
    pub return_type: String,
}

/// ClassHierarchy manages the inheritance graph and built-in classes
pub struct ClassHierarchy {
    classes: HashMap<String, ClassInfo>,
}

impl ClassHierarchy {
    pub fn new() -> Self {
        ClassHierarchy {
            classes: HashMap::new(),
        }
    }

    /// Add COOL's built-in classes: Object, IO, String, Int, Bool
    pub fn add_builtins(&mut self) {
        // Object is the root of the hierarchy
        self.add_builtin_class(
            "Object",
            None,
            vec![],
            vec![
                ("abort", vec![], "Object"),
                ("type_name", vec![], "String"),
                ("copy", vec![], "SELF_TYPE"),
            ],
        );

        // IO inherits from Object
        self.add_builtin_class(
            "IO",
            Some("Object"),
            vec![],
            vec![
                ("out_string", vec!["String"], "SELF_TYPE"),
                ("out_int", vec!["Int"], "SELF_TYPE"),
                ("in_string", vec![], "String"),
                ("in_int", vec![], "Int"),
            ],
        );

        // String inherits from Object
        self.add_builtin_class(
            "String",
            Some("Object"),
            vec![],
            vec![
                ("length", vec![], "Int"),
                ("concat", vec!["String"], "String"),
                ("substr", vec!["Int", "Int"], "String"),
            ],
        );

        // Int inherits from Object (no additional methods)
        self.add_builtin_class("Int", Some("Object"), vec![], vec![]);

        // Bool inherits from Object (no additional methods)
        self.add_builtin_class("Bool", Some("Object"), vec![], vec![]);
    }

    fn add_builtin_class(
        &mut self,
        name: &str,
        parent: Option<&str>,
        _attributes: Vec<(&str, &str)>,
        methods: Vec<(&str, Vec<&str>, &str)>,
    ) {
        let mut method_map = HashMap::new();
        for (method_name, param_types, return_type) in methods {
            method_map.insert(
                method_name.to_string(),
                MethodSignature {
                    name: method_name.to_string(),
                    param_types: param_types.iter().map(|s| s.to_string()).collect(),
                    return_type: return_type.to_string(),
                },
            );
        }

        let class_info = ClassInfo {
            name: name.to_string(),
            parent: parent.map(|s| s.to_string()),
            attributes: HashMap::new(),
            methods: method_map,
            is_builtin: true,
        };

        self.classes.insert(name.to_string(), class_info);
    }

    /// Build the class hierarchy from parsed classes
    pub fn build(&mut self, classes: &[Class]) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        // Check for duplicate class definitions
        let mut seen = HashSet::new();
        for class in classes {
            if self.classes.contains_key(&class.name) {
                errors.push(SemanticError::DuplicateClass {
                    name: class.name.clone(),
                    line: 0,
                });
            } else if !seen.insert(class.name.clone()) {
                errors.push(SemanticError::DuplicateClass {
                    name: class.name.clone(),
                    line: 0,
                });
            }
        }

        // Check for invalid inheritance (can't inherit from Int, String, Bool)
        for class in classes {
            if let Some(ref parent) = class.parent {
                if parent == "Int"
                    || parent == "String"
                    || parent == "Bool"
                    || parent == "SELF_TYPE"
                {
                    errors.push(SemanticError::InvalidInheritance {
                        class: class.name.clone(),
                        parent: parent.clone(),
                        line: 0,
                    });
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // Add classes to hierarchy with their methods and attributes
        for class in classes {
            let mut attributes = HashMap::new();
            let mut methods = HashMap::new();

            // Collect all attributes and methods from the class
            for feature in &class.features {
                match feature {
                    parser::ast::Feature::Attribute(attr) => {
                        attributes.insert(attr.name.clone(), attr.attr_type.clone());
                    }
                    parser::ast::Feature::Method(method) => {
                        let param_types = method.formals.iter().map(|f| f.typ.clone()).collect();
                        methods.insert(
                            method.name.clone(),
                            MethodSignature {
                                name: method.name.clone(),
                                param_types,
                                return_type: method.return_type.clone(),
                            },
                        );
                    }
                }
            }

            let class_info = ClassInfo {
                name: class.name.clone(),
                parent: class.parent.clone().or(Some("Object".to_string())),
                attributes,
                methods,
                is_builtin: false,
            };
            self.classes.insert(class.name.clone(), class_info);
        }

        Ok(())
    }

    /// Check for cyclic inheritance
    pub fn check_cycles(&self) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        // First check for undefined parent classes
        for (_class_name, class_info) in &self.classes {
            if let Some(ref parent) = class_info.parent {
                if !self.classes.contains_key(parent) {
                    errors.push(SemanticError::UndefinedType {
                        name: parent.clone(),
                        line: 0,
                    });
                }
            }
        }

        // Then check for cycles
        for class_name in self.classes.keys() {
            if let Some(cycle) = self.detect_cycle(class_name) {
                errors.push(SemanticError::CyclicInheritance { path: cycle });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn detect_cycle(&self, start: &str) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut current = start;

        loop {
            if !visited.insert(current) {
                // Found a cycle
                let cycle_start = path.iter().position(|c| c == current).unwrap_or(0);
                return Some(path[cycle_start..].to_vec());
            }

            path.push(current.to_string());

            if let Some(class_info) = self.classes.get(current) {
                if let Some(ref parent) = class_info.parent {
                    if !self.classes.contains_key(parent) {
                        // Undefined parent type
                        return None;
                    }
                    current = parent;
                } else {
                    // Reached root (Object)
                    return None;
                }
            } else {
                // Class not found
                return None;
            }
        }
    }

    /// Check if class1 conforms to (is a subtype of) class2
    pub fn conforms(&self, class1: &str, class2: &str) -> bool {
        if class1 == class2 {
            return true;
        }

        let mut current = class1;
        while let Some(class_info) = self.classes.get(current) {
            if let Some(ref parent) = class_info.parent {
                if parent == class2 {
                    return true;
                }
                current = parent;
            } else {
                break;
            }
        }

        false
    }

    /// Get the least upper bound (common ancestor) of two types
    pub fn lub(&self, type1: &str, type2: &str) -> Option<String> {
        let ancestors1 = self.get_ancestors(type1);
        let ancestors2 = self.get_ancestors(type2);

        // Find first common ancestor
        for ancestor in ancestors1 {
            if ancestors2.contains(&ancestor) {
                return Some(ancestor);
            }
        }

        None
    }

    fn get_ancestors(&self, class_name: &str) -> Vec<String> {
        let mut ancestors = vec![class_name.to_string()];
        let mut current = class_name;

        while let Some(class_info) = self.classes.get(current) {
            if let Some(ref parent) = class_info.parent {
                ancestors.push(parent.clone());
                current = parent;
            } else {
                break;
            }
        }

        ancestors
    }

    pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
        self.classes.get(name)
    }

    pub fn class_exists(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    /// Get the parent class name for a given class
    pub fn get_parent(&self, class_name: &str) -> Option<String> {
        self.classes
            .get(class_name)
            .and_then(|info| info.parent.clone())
    }

    /// Get all attributes for a class including inherited ones
    pub fn get_all_attributes(&self, class_name: &str) -> HashMap<String, String> {
        let mut attributes = HashMap::new();
        let mut current = class_name;

        // Walk up the inheritance chain collecting attributes
        let mut ancestors = Vec::new();
        while let Some(class_info) = self.classes.get(current) {
            ancestors.push(current);
            if let Some(ref parent) = class_info.parent {
                current = parent;
            } else {
                break;
            }
        }

        // Add attributes from parent to child (so child can override)
        for ancestor in ancestors.iter().rev() {
            if let Some(class_info) = self.classes.get(*ancestor) {
                for (attr_name, attr_type) in &class_info.attributes {
                    attributes.insert(attr_name.clone(), attr_type.clone());
                }
            }
        }

        attributes
    }

    /// Get the least upper bound of two types (wrapper for lub that returns a String)
    pub fn least_upper_bound(&self, t1: &str, t2: &str) -> String {
        self.lub(t1, t2).unwrap_or_else(|| "Object".to_string())
    }

    /// Get method information (including inherited methods)
    pub fn get_method(&self, class_name: &str, method_name: &str) -> Option<MethodSignature> {
        let mut current = class_name;
        loop {
            if let Some(class_info) = self.classes.get(current) {
                if let Some(method_info) = class_info.methods.get(method_name) {
                    return Some(method_info.clone());
                }
                if let Some(parent) = &class_info.parent {
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        None
    }

    /// Find which class defines a method (walking up inheritance chain)
    pub fn get_method_defining_class(&self, class_name: &str, method_name: &str) -> Option<String> {
        let mut current = class_name;
        loop {
            if let Some(class_info) = self.classes.get(current) {
                if class_info.methods.contains_key(method_name) {
                    return Some(current.to_string());
                }
                if let Some(parent) = &class_info.parent {
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        None
    }

    /// Get all methods for a class including inherited ones
    pub fn get_all_methods(&self, class_name: &str) -> Vec<(String, MethodSignature)> {
        let mut current = class_name;

        // Walk up the inheritance chain collecting methods
        let mut ancestors = Vec::new();
        while let Some(class_info) = self.classes.get(current) {
            ancestors.push(current);
            if let Some(ref parent) = class_info.parent {
                current = parent;
            } else {
                break;
            }
        }

        // Collect methods in vtable order: parent methods first, then child methods
        // Methods are sorted alphabetically within each class for determinism
        // Overridden methods keep their original slot
        let mut indexed: Vec<_> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for ancestor in ancestors.iter().rev() {
            if let Some(class_info) = self.classes.get(*ancestor) {
                let mut class_methods: Vec<_> = class_info.methods.iter().collect();
                class_methods.sort_by_key(|(name, _)| (*name).clone());

                for (method_name, method_sig) in class_methods {
                    if !seen.contains(method_name) {
                        seen.insert(method_name.clone());
                        indexed.push((method_name.clone(), method_sig.clone()));
                    }
                }
            }
        }

        indexed
    }
}

impl Default for ClassHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtins() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        assert!(hierarchy.class_exists("Object"));
        assert!(hierarchy.class_exists("IO"));
        assert!(hierarchy.class_exists("String"));
        assert!(hierarchy.class_exists("Int"));
        assert!(hierarchy.class_exists("Bool"));
    }

    #[test]
    fn test_conformance() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        assert!(hierarchy.conforms("IO", "Object"));
        assert!(hierarchy.conforms("String", "Object"));
        assert!(!hierarchy.conforms("Object", "IO"));
        assert!(hierarchy.conforms("Int", "Int"));
    }

    #[test]
    fn test_lub() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        assert_eq!(hierarchy.lub("IO", "String"), Some("Object".to_string()));
        assert_eq!(hierarchy.lub("Int", "Int"), Some("Int".to_string()));
        assert_eq!(hierarchy.lub("IO", "Object"), Some("Object".to_string()));
    }

    #[test]
    fn test_method_inheritance() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        // IO should inherit methods from Object
        assert!(hierarchy.get_method("IO", "abort").is_some());
        assert!(hierarchy.get_method("IO", "type_name").is_some());
        assert!(hierarchy.get_method("IO", "copy").is_some());

        // And have its own methods
        assert!(hierarchy.get_method("IO", "out_string").is_some());
        assert!(hierarchy.get_method("IO", "in_int").is_some());
    }

    #[test]
    fn test_add_custom_class() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        let class = Class {
            name: "MyClass".to_string(),
            parent: Some("Object".to_string()),
            features: vec![],
        };

        assert!(hierarchy.build(&[class]).is_ok());
        assert!(hierarchy.class_exists("MyClass"));
        assert!(hierarchy.conforms("MyClass", "Object"));
    }

    #[test]
    fn test_cannot_inherit_from_basic_classes() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        let class = Class {
            name: "MyInt".to_string(),
            parent: Some("Int".to_string()),
            features: vec![],
        };

        // Build should succeed, but check_cycles should fail
        let build_result = hierarchy.build(&[class]);
        if build_result.is_ok() {
            let result = hierarchy.check_cycles();
            assert!(result.is_err());
        } else {
            // If build itself fails (which is fine), that's also an error
            assert!(build_result.is_err());
        }
    }

    #[test]
    fn test_undefined_parent_class() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        let class = Class {
            name: "MyClass".to_string(),
            parent: Some("UndefinedClass".to_string()),
            features: vec![],
        };

        // Build should succeed, but check_cycles should fail
        let build_result = hierarchy.build(&[class]);
        if build_result.is_ok() {
            let result = hierarchy.check_cycles();
            assert!(result.is_err());
        } else {
            // If build itself fails, that's also an error detection
            assert!(build_result.is_err());
        }
    }

    #[test]
    fn test_cycle_detection() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        // This would create a cycle if we could manipulate the hierarchy directly
        // For now, we test that the check_cycles method exists and runs
        assert!(hierarchy.check_cycles().is_ok());
    }

    #[test]
    fn test_duplicate_class() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        let class = Class {
            name: "Object".to_string(),
            parent: None,
            features: vec![],
        };

        let result = hierarchy.build(&[class]);
        assert!(result.is_err());
    }

    #[test]
    fn test_least_upper_bound_wrapper() {
        let mut hierarchy = ClassHierarchy::new();
        hierarchy.add_builtins();

        assert_eq!(hierarchy.least_upper_bound("IO", "String"), "Object");
        assert_eq!(hierarchy.least_upper_bound("Int", "Int"), "Int");
        assert_eq!(hierarchy.least_upper_bound("Bool", "String"), "Object");
    }
}
