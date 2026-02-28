/// High-level IR (HIR) - Typed AST with resolved types and method dispatch information
/// Type ID representing a resolved COOL type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeId {
    Object,
    IO,
    String,
    Int,
    Bool,
    Class(String),
    SelfType(String), // SELF_TYPE with the current class context
    NoType,           // For error recovery
}

impl TypeId {
    pub fn from_string(s: &str, current_class: Option<&str>) -> Self {
        match s {
            "Object" => TypeId::Object,
            "IO" => TypeId::IO,
            "String" => TypeId::String,
            "Int" => TypeId::Int,
            "Bool" => TypeId::Bool,
            "SELF_TYPE" => {
                if let Some(class) = current_class {
                    TypeId::SelfType(class.to_string())
                } else {
                    TypeId::NoType
                }
            }
            _ => TypeId::Class(s.to_string()),
        }
    }
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeId::Object => write!(f, "Object"),
            TypeId::IO => write!(f, "IO"),
            TypeId::String => write!(f, "String"),
            TypeId::Int => write!(f, "Int"),
            TypeId::Bool => write!(f, "Bool"),
            TypeId::Class(name) => write!(f, "{}", name),
            TypeId::SelfType(class) => write!(f, "SELF_TYPE({})", class),
            TypeId::NoType => write!(f, "_no_type"),
        }
    }
}

/// Method dispatch information with resolved class and method index
#[derive(Debug, Clone)]
pub struct DispatchInfo {
    pub class_name: String,
    pub method_name: String,
    pub method_index: usize, // VTable slot index
}

/// HIR Program
#[derive(Debug, Clone)]
pub struct HirProgram {
    pub classes: Vec<HirClass>,
}

/// HIR Class
#[derive(Debug, Clone)]
pub struct HirClass {
    pub name: String,
    pub parent: Option<String>,
    pub class_tag: usize, // Unique integer for runtime type checks
    pub attributes: Vec<HirAttribute>,
    pub methods: Vec<HirMethod>,
}

/// HIR Attribute
#[derive(Debug, Clone)]
pub struct HirAttribute {
    pub name: String,
    pub typ: TypeId,
    pub init: Option<HirExpr>,
    pub offset: usize, // Byte offset in object layout
}

/// HIR Method
#[derive(Debug, Clone)]
pub struct HirMethod {
    pub name: String,
    pub formals: Vec<HirFormal>,
    pub return_type: TypeId,
    pub body: HirExpr,
    pub vtable_index: usize, // VTable slot for this method
}

/// HIR Formal parameter
#[derive(Debug, Clone)]
pub struct HirFormal {
    pub name: String,
    pub typ: TypeId,
}

/// HIR Expression with type information
#[derive(Debug, Clone)]
pub enum HirExpr {
    /// Assignment: name := expr
    Assign {
        name: String,
        expr: Box<HirExpr>,
        typ: TypeId,
    },

    /// Dynamic dispatch: expr.method(args)
    Dispatch {
        object: Box<HirExpr>,
        dispatch_info: DispatchInfo,
        args: Vec<HirExpr>,
        typ: TypeId,
    },

    /// Static dispatch: expr@Type.method(args)
    StaticDispatch {
        object: Box<HirExpr>,
        type_name: String,
        dispatch_info: DispatchInfo,
        args: Vec<HirExpr>,
        typ: TypeId,
    },

    /// Conditional: if cond then then_branch else else_branch
    If {
        cond: Box<HirExpr>,
        then_branch: Box<HirExpr>,
        else_branch: Box<HirExpr>,
        typ: TypeId,
    },

    /// Loop: while cond loop body pool
    While {
        cond: Box<HirExpr>,
        body: Box<HirExpr>,
        typ: TypeId,
    },

    /// Block: { expr1; expr2; ...; exprN }
    Block { exprs: Vec<HirExpr>, typ: TypeId },

    /// Let binding: let name: type <- init in body
    Let {
        name: String,
        typ: TypeId,
        init: Box<HirExpr>,
        body: Box<HirExpr>,
        result_type: TypeId,
    },

    /// Case expression
    Case {
        expr: Box<HirExpr>,
        branches: Vec<CaseBranch>,
        typ: TypeId,
    },

    /// Object creation: new Type
    New { type_name: String, typ: TypeId },

    /// Unary negation: ~expr
    IsVoid { expr: Box<HirExpr>, typ: TypeId },

    /// Arithmetic negation: -expr
    Negate { expr: Box<HirExpr>, typ: TypeId },

    /// Arithmetic: expr1 + expr2
    Add {
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        typ: TypeId,
    },

    /// Arithmetic: expr1 - expr2
    Sub {
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        typ: TypeId,
    },

    /// Arithmetic: expr1 * expr2
    Mul {
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        typ: TypeId,
    },

    /// Arithmetic: expr1 / expr2
    Div {
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        typ: TypeId,
    },

    /// Comparison: expr1 < expr2
    Lt {
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        typ: TypeId,
    },

    /// Comparison: expr1 <= expr2
    Le {
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        typ: TypeId,
    },

    /// Comparison: expr1 = expr2
    Eq {
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        typ: TypeId,
    },

    /// Boolean negation: not expr
    Not { expr: Box<HirExpr>, typ: TypeId },

    /// Integer literal
    IntLiteral { value: i32, typ: TypeId },

    /// String literal
    StringLiteral { value: String, typ: TypeId },

    /// Boolean literal
    BoolLiteral { value: bool, typ: TypeId },

    /// Variable reference
    Object { name: String, typ: TypeId },
}

impl HirExpr {
    /// Get the type of this expression
    pub fn get_type(&self) -> &TypeId {
        match self {
            HirExpr::Assign { typ, .. } => typ,
            HirExpr::Dispatch { typ, .. } => typ,
            HirExpr::StaticDispatch { typ, .. } => typ,
            HirExpr::If { typ, .. } => typ,
            HirExpr::While { typ, .. } => typ,
            HirExpr::Block { typ, .. } => typ,
            HirExpr::Let { result_type, .. } => result_type,
            HirExpr::Case { typ, .. } => typ,
            HirExpr::New { typ, .. } => typ,
            HirExpr::IsVoid { typ, .. } => typ,
            HirExpr::Negate { typ, .. } => typ,
            HirExpr::Add { typ, .. } => typ,
            HirExpr::Sub { typ, .. } => typ,
            HirExpr::Mul { typ, .. } => typ,
            HirExpr::Div { typ, .. } => typ,
            HirExpr::Lt { typ, .. } => typ,
            HirExpr::Le { typ, .. } => typ,
            HirExpr::Eq { typ, .. } => typ,
            HirExpr::Not { typ, .. } => typ,
            HirExpr::IntLiteral { typ, .. } => typ,
            HirExpr::StringLiteral { typ, .. } => typ,
            HirExpr::BoolLiteral { typ, .. } => typ,
            HirExpr::Object { typ, .. } => typ,
        }
    }
}

/// Case branch
#[derive(Debug, Clone)]
pub struct CaseBranch {
    pub name: String,
    pub type_name: String,
    pub expr: HirExpr,
}
