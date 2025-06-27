pub type Span = std::ops::Range<usize>;

#[derive(Debug, Clone)]
pub enum Expr {
    Ident(String),
    Number(i64),
    Binary(Box<Expr>, Operator, Box<Expr>),
    Dummy,
}

#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub struct Feature {
    // For now, we only support methods
    pub name: String,
    pub formals: Vec<(String, String)>,
    pub return_type: String,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub inherits: Option<String>,
    pub features: Vec<Feature>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub classes: Vec<Class>,
}
