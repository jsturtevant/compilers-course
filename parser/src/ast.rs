#![allow(unused)]
use std::boxed::Box;

#[derive(Debug, Clone)]
pub struct Program {
    pub classes: Vec<Class>,
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub parent: Option<String>,
    pub features: Vec<Feature>,
}

#[derive(Debug, Clone)]
pub enum Feature {
    Method(MethodFeature),
    Attribute(AttributeFeature),
}

#[derive(Debug, Clone)]
pub struct MethodFeature {
    pub name: String,
    pub formals: Vec<Formal>,
    pub return_type: String,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct AttributeFeature {
    pub name: String,
    pub attr_type: String,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct Formal {
    pub name: String,
    pub typ: String,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Assign {
        name: String,
        expr: Box<Expr>,
    },
    Dispatch {
        expr: Box<Expr>,
        static_type: Option<String>,
        method: String,
        args: Vec<Expr>,
    },
    FuncCall {
        name: String,
        args: Vec<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    Block(Vec<Expr>),
    Let {
        bindings: Vec<LetBinding>,
        body: Box<Expr>,
    },
    Case {
        expr: Box<Expr>,
        branches: Vec<CaseBranch>,
    },
    New(String),
    IsVoid(Box<Expr>),
    Plus(Box<Expr>, Box<Expr>),
    Minus(Box<Expr>, Box<Expr>),
    Times(Box<Expr>, Box<Expr>),
    Divide(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Paren(Box<Expr>),
    Id(String),
    Integer(i32),
    String(String),
    True,
    False,
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub typ: String,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct CaseBranch {
    pub name: String,
    pub typ: String,
    pub expr: Expr,
}
