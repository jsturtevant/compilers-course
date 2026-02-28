use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ScopeStack {
    scopes: Vec<HashMap<String, String>>,
}

impl ScopeStack {
    pub fn new() -> Self {
        ScopeStack {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope for let, case branches, or method bodies
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Look up a variable, searching from innermost scope outward
    pub fn lookup_variable(&self, name: &str) -> Option<&String> {
        for scope in self.scopes.iter().rev() {
            if let Some(typ) = scope.get(name) {
                return Some(typ);
            }
        }
        None
    }

    /// Add a variable to the current scope
    pub fn add_variable(&mut self, name: String, typ: String) -> Result<(), String> {
        if let Some(current_scope) = self.scopes.last_mut() {
            if current_scope.contains_key(&name) {
                return Err(format!("Variable '{}' already defined in current scope", name));
            }
            current_scope.insert(name, typ);
            Ok(())
        } else {
            Err("No active scope".to_string())
        }
    }

    /// Add 'self' to the current scope with SELF_TYPE
    pub fn add_self(&mut self, class_name: &str) {
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.insert("self".to_string(), class_name.to_string());
        }
    }

    /// Get the number of scopes
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_variable_lookup() {
        let mut scope = ScopeStack::new();
        scope.add_variable("x".to_string(), "Int".to_string()).unwrap();
        assert_eq!(scope.lookup_variable("x"), Some(&"Int".to_string()));
    }

    #[test]
    fn test_nested_scopes() {
        let mut scope = ScopeStack::new();
        scope.add_variable("x".to_string(), "Int".to_string()).unwrap();
        
        scope.push_scope();
        scope.add_variable("y".to_string(), "String".to_string()).unwrap();
        
        // Both variables visible in inner scope
        assert_eq!(scope.lookup_variable("x"), Some(&"Int".to_string()));
        assert_eq!(scope.lookup_variable("y"), Some(&"String".to_string()));
        
        scope.pop_scope();
        
        // Only x visible after popping
        assert_eq!(scope.lookup_variable("x"), Some(&"Int".to_string()));
        assert_eq!(scope.lookup_variable("y"), None);
    }

    #[test]
    fn test_shadowing() {
        let mut scope = ScopeStack::new();
        scope.add_variable("x".to_string(), "Int".to_string()).unwrap();
        
        scope.push_scope();
        scope.add_variable("x".to_string(), "String".to_string()).unwrap();
        
        // Inner x shadows outer x
        assert_eq!(scope.lookup_variable("x"), Some(&"String".to_string()));
        
        scope.pop_scope();
        
        // Back to original x
        assert_eq!(scope.lookup_variable("x"), Some(&"Int".to_string()));
    }

    #[test]
    fn test_duplicate_in_same_scope() {
        let mut scope = ScopeStack::new();
        scope.add_variable("x".to_string(), "Int".to_string()).unwrap();
        
        let result = scope.add_variable("x".to_string(), "String".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_self_binding() {
        let mut scope = ScopeStack::new();
        scope.add_self("MyClass");
        
        assert_eq!(scope.lookup_variable("self"), Some(&"MyClass".to_string()));
    }

    #[test]
    fn test_cannot_pop_root_scope() {
        let mut scope = ScopeStack::new();
        assert_eq!(scope.depth(), 1);
        
        scope.pop_scope();
        assert_eq!(scope.depth(), 1); // Still 1, cannot pop root
    }
}
