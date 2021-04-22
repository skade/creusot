use std::collections::HashSet;
use std::collections::HashMap;
use std::fmt::Display;

pub mod printer;

pub fn drop_fix() -> QName {
    QName { module: vec![], name: vec!["drop_fix".into()] }
}
pub fn drop_uint() -> QName {
    QName { module: vec![], name: vec!["drop_uint".into()] }
}
pub fn drop_int() -> QName {
    QName { module: vec![], name: vec!["drop_int".into()] }
}
pub fn drop_float() -> QName {
    QName { module: vec![], name: vec!["drop_float".into()] }
}
pub fn drop_bool() -> QName {
    QName { module: vec![], name: vec!["drop_bool".into()] }
}
pub fn drop_mut_ref() -> QName {
    QName { module: vec![], name: vec!["drop_mut_ref".into()] }
}
pub fn drop_ref() -> QName {
    QName { module: vec![], name: vec!["drop_ref".into()] }
}

#[derive(Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct BlockId(pub usize);

#[derive(Debug)]
pub enum Terminator {
    Goto(BlockId),
    Absurd,
    Return,
    Switch(Exp, Vec<(Pattern, Terminator)>),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assign { lhs: LocalIdent, rhs: Exp },
    Invariant(String, Exp),
    Assume(Exp),
    Assert(Exp),
}

#[derive(Debug, Clone)]
pub enum Type {
    Bool,
    Char,
    Integer,
    MutableBorrow(Box<Type>),
    TVar(String),
    TConstructor(QName),
    TApp(Box<Type>, Vec<Type>),
    Tuple(Vec<Type>),
    TFun(Box<Type>, Box<Type>),
}

impl Type {
    pub fn predicate(ty: Self) -> Self {
        Self::TFun(box ty, box Self::Bool)
    }

    fn complex(&self) -> bool {
        use Type::*;
        !matches!(
            self,
            Bool | Char | Integer | TVar(_) | Tuple(_) | TConstructor(_)
        )
    }

    fn find_used_types(&self, tys: &mut HashSet<QName>) {
        use Type::*;

        match self {
            MutableBorrow(t) => t.find_used_types(tys),
            TConstructor(qn) => {
                tys.insert(qn.clone());
            }
            TApp(f, args) => {
                f.find_used_types(tys);
                args.iter().for_each(|arg| arg.find_used_types(tys));
            }
            Tuple(args) => {
                args.iter().for_each(|arg| arg.find_used_types(tys));
            }
            TFun(a, b) => {
                a.find_used_types(tys);
                b.find_used_types(tys);
            }
            _ => (),
        }
    }
}

#[derive(Debug)]
pub struct TyDecl {
    pub ty_name: QName,
    pub ty_params: Vec<String>,
    pub ty_constructors: Vec<(String, Vec<Type>)>,
}

impl TyDecl {
    pub fn used_types(&self) -> HashSet<QName> {
        let mut used = HashSet::new();
        for (_, var_decl) in &self.ty_constructors {
            for ty in var_decl {
                ty.find_used_types(&mut used);
            }
        }
        used
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LocalIdent {
    /// A MIR local along with an optional human-readable name
    Anon(usize, Option<String>),

    /// A local variable,
    Name(String),
}

impl From<&str> for LocalIdent {
    fn from(s: &str) -> Self {
        Self::Name(s.to_owned())
    }
}

impl From<String> for LocalIdent {
    fn from(s: String) -> Self {
        Self::Name(s)
    }
}

impl From<LocalIdent> for Exp {
    fn from(li: LocalIdent) -> Self {
        Exp::Var(li)
    }
}

impl Display for LocalIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalIdent::Anon(l, n) => {
                if let Some(n) = n {
                    write!(f, "{}", n)?;
                }
                write!(f, "_{:?}", l)
            }
            LocalIdent::Name(nm) => write!(f, "{}", nm),
        }
    }
}

use itertools::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QName {
    pub module: Vec<String>,
    // TODO: get rid of the vec here!
    pub name: Vec<String>,
}

impl QName {
    pub fn name(&self) -> String {
        format!("{}", self.name.iter().format("_"))
    }
}

impl From<&str> for QName {
    fn from(nm: &str) -> Self {
        QName { module: vec![], name: vec![nm.to_string()] }
    }
}

#[derive(Debug, Clone)]
pub enum BinOp {
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    Ne,
}

#[derive(Debug, Clone)]
pub enum UnOp { Not, Neg }

#[derive(Debug, Clone)]
pub enum Exp {
    Current(Box<Exp>),
    Final(Box<Exp>),
    Let { pattern: Pattern, arg: Box<Exp>, body: Box<Exp> },
    Var(LocalIdent),
    QVar(QName),
    RecUp { record: Box<Exp>, label: String, val: Box<Exp> },
    RecField { record: Box<Exp>, label: String },
    Tuple(Vec<Exp>),
    Constructor { ctor: QName, args: Vec<Exp> },
    BorrowMut(Box<Exp>),
    Const(Constant),
    BinaryOp(BinOp, Box<Exp>, Box<Exp>),
    UnaryOp(UnOp, Box<Exp>),
    Call(Box<Exp>, Vec<Exp>),
    Verbatim(String),
    // Seq(Box<Exp>, Box<Exp>),
    Abs(LocalIdent, Box<Exp>),
    Match(Box<Exp>, Vec<(Pattern, Exp)>),

    // Predicates
    Absurd,
    Impl(Box<Exp>, Box<Exp>),
    Forall(Vec<(LocalIdent, Type)>, Box<Exp>),
    Exists(Vec<(LocalIdent, Type)>, Box<Exp>),
}

impl Exp {
    pub fn conj(l: Exp, r: Exp) -> Self {
        Exp::BinaryOp(BinOp::And, box l, box r)
    }

    pub fn mk_true() -> Self {
        Exp::Const(Constant::const_true())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Closed,
    Any,
    Let,
    Assign,
    Impl,
    Or,
    And,
    Compare,
    AddSub,
    Mul,
    PrefixOp,
    Term,
    Call,
}

impl Exp {
    fn precedence(&self) -> Precedence {
        use Precedence::*;

        match self {
            Exp::Current(_) => PrefixOp,
            Exp::Final(_) => PrefixOp,
            Exp::Let { .. } => Let,
            Exp::Abs(_, _) => Let,
            Exp::Var(_) => Closed,
            Exp::QVar(_) => Closed,
            Exp::RecUp { .. } => Term,
            Exp::RecField { .. } => Any,
            Exp::Tuple(_) => Closed,
            Exp::Constructor { .. } => Term,
            // Exp::Seq(_, _) => { Term }
            Exp::Match(_, _) => Term,
            Exp::BorrowMut(_) => Term,
            Exp::Const(_) => Closed,
            Exp::UnaryOp(UnOp::Neg, _) => PrefixOp,
            Exp::UnaryOp(UnOp::Not, _) => Call,
            Exp::BinaryOp(op, _, _) => {
                match op {
                    BinOp::And => And,
                    BinOp::Or => Or,
                    BinOp::Add => AddSub,
                    BinOp::Sub => AddSub,
                    BinOp::Mul => Mul,
                    BinOp::Div => Term,
                    BinOp::Eq => Compare,
                    BinOp::Lt => Compare,
                    BinOp::Le => Compare,
                    BinOp::Ne => Compare,
                    BinOp::Ge => Compare,
                    BinOp::Gt => Compare,
                }
            }
            Exp::Call(_, _) => Call,
            Exp::Verbatim(_) => Any,
            Exp::Impl(_, _) => Impl,
            Exp::Forall(_, _) => Any,
            Exp::Exists(_, _) => Any,
            Exp::Absurd => Closed,
        }
    }

    pub fn fvs(&self) -> HashSet<LocalIdent> {
        match self {
            Exp::Current(e) => e.fvs(),
            Exp::Final(e) => e.fvs(),
            Exp::Let { pattern, arg, body } => {
                let bound = pattern.binders();

                &(&body.fvs() - &bound) | &arg.fvs()
            }
            Exp::Var(v) => {
                let mut fvs = HashSet::new();
                fvs.insert(v.clone());
                fvs
            }
            Exp::QVar(_) => HashSet::new(),
            // Exp::RecUp { record, label, val } => {}
            // Exp::Tuple(_) => {}
            Exp::Constructor { ctor: _, args } => {
                args.iter().fold(HashSet::new(), |acc, v| &acc | &v.fvs())
            }
            Exp::Const(_) => HashSet::new(),
            Exp::BinaryOp(_, l, r) => &l.fvs() | &r.fvs(),
            Exp::Call(f, args) => args.iter().fold(f.fvs(), |acc, a| &acc | &a.fvs()),
            Exp::Impl(h, c) => &h.fvs() | &c.fvs(),
            Exp::Forall(bnds, exp) => bnds.iter().fold(exp.fvs(), |mut acc, (l, _)| {
                acc.remove(l);
                acc
            }),
            Exp::BorrowMut(e) => e.fvs(),
            Exp::Verbatim(_) => HashSet::new(),
            _ => unimplemented!(),
        }
    }

    pub fn subst(&mut self, subst: &HashMap<LocalIdent, Exp>) {
        match self {
            Exp::Current(e) => e.subst(subst),
            Exp::Final(e) => e.subst(subst),
            Exp::Let { pattern, arg, body } => {
                arg.subst(subst);
                let mut bound = pattern.binders();
                let mut subst = subst.clone();
                bound.drain().for_each(|k| {
                    subst.remove(&k);
                });

                body.subst(&subst);
            }
            Exp::Var(v) => {
                if let Some(e) = subst.get(v) {
                    *self = e.clone()
                }
            }
            Exp::RecUp { record, val, .. } => {
                record.subst(subst);
                val.subst(subst);
            }
            Exp::RecField { record, .. } => {
                record.subst(subst);
            }
            Exp::Tuple(tuple) => {
                for t in tuple {
                    t.subst(subst);
                }
            }
            Exp::Constructor { args, .. } => {
                for a in args {
                    a.subst(subst);
                }
            }
            Exp::Abs(ident, body) => {
                let mut subst = subst.clone();
                subst.remove(ident);
                body.subst(&subst);
            }
            Exp::Match(box scrut, brs) => {
                scrut.subst(subst);

                for (pat, br) in brs {
                    let mut s = subst.clone();
                    pat.binders().drain().for_each(|b| {
                        s.remove(&b);
                    });
                    br.subst(&s);
                }
            }
            Exp::BorrowMut(e) => e.subst(subst),
            Exp::UnaryOp(_, o) => {
                o.subst(subst);
            }
            Exp::BinaryOp(_, l, r) => {
                l.subst(subst);
                r.subst(subst)
            }
            Exp::Impl(hyp, exp) => {
                hyp.subst(subst);
                exp.subst(subst)
            }
            Exp::Forall(binders, exp) => {
                let mut subst = subst.clone();
                binders.iter().for_each(|k| {
                    subst.remove(&k.0);
                });
                exp.subst(&subst);
            }
            Exp::Exists(binders, exp) => {
                let mut subst = subst.clone();
                binders.iter().for_each(|k| {
                    subst.remove(&k.0);
                });
                exp.subst(&subst);
            }
            Exp::Call(_, a) => {
                for arg in a {
                    arg.subst(subst);
                }
            }
            Exp::QVar(_) => {}
            Exp::Const(_) => {}
            Exp::Verbatim(_) => {}
            Exp::Absurd => {}
        }
    }

    // Construct an application from this expression and an argument
    pub fn app_to(mut self, arg: Self) -> Self {
        match self {
            Exp::Call(_, ref mut args) => args.push(arg),
            _ => self = Exp::Call(box self, vec![arg]),
        }
        self
    }
}

#[derive(Debug, Clone)]
pub enum Constant {
    Int(i128, Option<Type>),
    Uint(u128,  Option<Type>),
    // Float(f64),
    Other(String),
}
impl Constant {
    pub fn const_true() -> Self {
        Constant::Other("true".to_owned())
    }
    pub fn const_false() -> Self {
        Constant::Other("false".to_owned())
    }
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Wildcard,
    VarP(LocalIdent),
    TupleP(Vec<Pattern>),
    ConsP(QName, Vec<Pattern>),
    // RecP(String, String),
}

impl Pattern {
    pub fn mk_true() -> Self {
        Self::ConsP(QName { module: vec![], name: vec!["True".into()] }, vec![])
    }

    pub fn mk_false() -> Self {
        Self::ConsP(QName { module: vec![], name: vec!["False".into()] }, vec![])
    }

    pub fn binders(&self) -> HashSet<LocalIdent> {
        match self {
            Pattern::Wildcard => HashSet::new(),
            Pattern::VarP(s) => {
                let mut b = HashSet::new();
                b.insert(s.clone());
                b
            }
            Pattern::TupleP(pats) => {
                pats.iter().map(|p| p.binders()).fold(HashSet::new(), |mut set, x| {
                    set.extend(x);
                    set
                })
            }
            Pattern::ConsP(_, args) => {
                args.iter().map(|p| p.binders()).fold(HashSet::new(), |mut set, x| {
                    set.extend(x);
                    set
                })
            }
        }
    }
}
