use crate::mlcfg::QName;
use rustc_hir::def_id::DefId;
use pearlite::term::{self, DerefKind, RefKind};
use pearlite::term::Name;
use crate::mlcfg::{self, Exp};
use crate::translation::ty::Ctx;

pub fn lower_term_to_why(ctx: &mut Ctx, t: term::Term) -> Exp {
    use term::Term::*;
    match t {
        Match { box expr, arms } => Exp::Match(
            box lower_term_to_why(ctx, expr),
            arms.into_iter().map(|t| lower_arm_to_why(ctx, t)).collect(),
        ),
        Binary { box left, op: pearlite::term::BinOp::Impl, box right } => {
            Exp::Impl(box lower_term_to_why(ctx, left), box lower_term_to_why(ctx, right))
        }
        Binary { box left, op, box right } => {
            let op = op_to_op(op);
            Exp::BinaryOp(op, box lower_term_to_why(ctx, left), box lower_term_to_why(ctx, right))
        }
        Unary { op, box expr } => {
            let expr = box lower_term_to_why(ctx, expr);
            match op {
                term::UnOp::Final => Exp::Final(expr),
                term::UnOp::Deref(Some(DerefKind::Ref(RefKind::Mut))) => Exp::Current(expr),
                term::UnOp::Deref(Some(_)) => *expr,
                term::UnOp::Deref(None) => unreachable!(),
                term::UnOp::Neg => Exp::UnaryOp(rustc_middle::mir::UnOp::Neg, expr),
                term::UnOp::Not => Exp::UnaryOp(rustc_middle::mir::UnOp::Not, expr),
            }
        }
        Variable { path } => match path {
            Name::Path { .. } => Exp::QVar(lower_value_path(ctx, path)),
            Name::Ident(i) => Exp::Var(i.into()),
        },
        Call { func, args } => {
            let is_c = is_constructor(ctx, &func);
            let name = lower_value_path(ctx, func);
            let args = args.into_iter().map(|t| lower_term_to_why(ctx, t)).collect();

            if is_c {
                Exp::Constructor { ctor: name, args }
            } else {
                Exp::Call(box Exp::QVar(name), args)
            }
        }
        Lit { lit } => Exp::Const(lit_to_const(lit)),
        Forall { args, box body } => {
            let args = args.into_iter().map(|(i, t)| (i.0.into(), lower_type_to_why(ctx, t))).collect();

            Exp::Forall(args, box lower_term_to_why(ctx, body))
        }
        Exists { args, box body } => {
            let args = args.into_iter().map(|(i, t)| (i.0.into(), lower_type_to_why(ctx, t))).collect();

            Exp::Exists(args, box lower_term_to_why(ctx, body))
        }
        Let { pat, box arg, box body } => Exp::Let {
            pattern: lower_pattern_to_why(ctx, pat),
            arg: box lower_term_to_why(ctx, arg),
            body: box lower_term_to_why(ctx, body),
        },
        Absurd => Exp::Absurd,
        Cast { box expr, ty: _ } => lower_term_to_why(ctx, expr),
        Tuple { elems } => {
            Exp::Tuple(elems.into_iter().map(|t| lower_term_to_why(ctx, t)).collect())
        }
        If { box cond, box then_branch, box else_branch } => {
            use mlcfg::Pattern;
            Exp::Match(box lower_term_to_why(ctx, cond), vec![
                (Pattern::mk_true(), lower_term_to_why(ctx, then_branch)),
                (Pattern::mk_false(), lower_term_to_why(ctx, else_branch)),
            ])
        }
    }
}

pub fn lower_type_to_why(ctx: &mut Ctx, ty: pearlite::term::Type) -> crate::mlcfg::Type {
    use crate::mlcfg::Type::*;
    use pearlite::term::*;

    match ty {
        term::Type::Path { path } => TConstructor(lower_type_path(ctx, path)),
        term::Type::Box { box ty } => lower_type_to_why(ctx, ty),
        term::Type::Reference { kind: RefKind::Mut, box ty } => {
            MutableBorrow(box lower_type_to_why(ctx, ty))
        }
        term::Type::Reference { kind: _, box ty } => lower_type_to_why(ctx, ty),
        term::Type::Tuple { elems } => Tuple(elems.into_iter().map(|t| lower_type_to_why(ctx, t)).collect()),
        term::Type::Lit(lit) => {
            use pearlite::term::Size::*;
            use rustc_middle::ty::{FloatTy::*, IntTy::*, UintTy::*};

            match lit {
                term::LitTy::Signed(s) => match s {
                    Eight => Int(I8),
                    Sixteen => Int(I16),
                    ThirtyTwo => Int(I32),
                    SixtyFour => Int(I64),
                    Mach => Int(Isize),
                    Unknown => {
                        unimplemented!("integers")
                    }
                },
                term::LitTy::Unsigned(s) => match s {
                    Eight => Uint(U8),
                    Sixteen => Uint(U16),
                    ThirtyTwo => Uint(U32),
                    SixtyFour => Uint(U64),
                    Mach => Uint(Usize),
                    Unknown => {
                        unimplemented!("uintegers")
                    }
                },
                term::LitTy::Integer => Integer,
                term::LitTy::Float => Float(F32),
                term::LitTy::Double => Float(F64),
                term::LitTy::Boolean => Bool,
            }
        }
        term::Type::App { box func, args } => {
            TApp(box lower_type_to_why(ctx, func), args.into_iter().map(|t| lower_type_to_why(ctx, t)).collect())
        }
        term::Type::Function { args, box res } => args
            .into_iter()
            .rfold(lower_type_to_why(ctx, res), |acc, arg| TFun(box lower_type_to_why(ctx, arg), box acc)),
        term::Type::Var(tyvar) => TVar(('a'..).nth(tyvar.0 as usize).unwrap().to_string()),
        term::Type::Unknown(_) => {
            panic!()
        } // _ => panic!("{:?}", ty),
    }
}

fn lit_to_const(lit: pearlite::term::Literal) -> crate::mlcfg::Constant {
    use crate::mlcfg::Constant::{self, *};
    use rustc_middle::ty::UintTy::*;

    match lit {
        term::Literal::U8(u) => Uint(u as u128, Some(U8)),
        term::Literal::U16(u) => Uint(u as u128, Some(U16)),
        term::Literal::U32(u) => Uint(u as u128, Some(U32)),
        term::Literal::U64(u) => Uint(u as u128, Some(U64)),
        term::Literal::Usize(u) => Uint(u as u128, Some(Usize)),
        term::Literal::Int(u) => Int(u as i128, None),
        term::Literal::F32(_) => {
            unimplemented!()
        }
        term::Literal::F64(_) => {
            unimplemented!()
        }
        term::Literal::Bool(b) => {
            if b {
                Constant::const_true()
            } else {
                Constant::const_false()
            }
        }
    }
}

fn op_to_op(op: term::BinOp) -> mlcfg::FullBinOp {
    use mlcfg::FullBinOp::*;
    use rustc_middle::mir::BinOp;
    match op {
        term::BinOp::Add => Other(BinOp::Add),
        term::BinOp::Sub => Other(BinOp::Sub),
        term::BinOp::Mul => Other(BinOp::Mul),
        term::BinOp::Div => Other(BinOp::Div),
        term::BinOp::Eq => Other(BinOp::Eq),
        term::BinOp::Ne => Other(BinOp::Ne),
        term::BinOp::Le => Other(BinOp::Le),
        term::BinOp::Ge => Other(BinOp::Ge),
        term::BinOp::Gt => Other(BinOp::Gt),
        term::BinOp::Lt => Other(BinOp::Lt),
        term::BinOp::Rem => Other(BinOp::Rem),
        term::BinOp::And => And,
        term::BinOp::Or => Or,
        term::BinOp::Impl => {
            panic!()
        }
    }
}

fn lower_arm_to_why(ctx: &mut Ctx, a: term::MatchArm) -> (mlcfg::Pattern, Exp) {
    (lower_pattern_to_why(ctx, a.pat), lower_term_to_why(ctx, *a.body))
}

fn lower_pattern_to_why(ctx: &mut Ctx, p: term::Pattern) -> mlcfg::Pattern {
    use mlcfg::Pattern;
    match p {
        term::Pattern::Var(x) => Pattern::VarP(x.0.into()),
        // term::Pattern::Struct { path, fields } => {}
        term::Pattern::TupleStruct { path, fields } => {
            let name = lower_value_path(ctx, path);
            let fields = fields.into_iter().map(|p| lower_pattern_to_why(ctx, p)).collect();

            Pattern::ConsP(name, fields)
        }
        term::Pattern::Boolean(b) => {
            if b {
                Pattern::mk_true()
            } else {
                Pattern::mk_false()
            }
        }
        term::Pattern::Wild => Pattern::Wildcard,
        _ => {
            unimplemented!()
        }
    }
}

fn is_constructor(ctx: &mut Ctx, path: &Name) -> bool {
    match path {
        Name::Ident(_) => false,
        Name::Path { id, ..} => {
            let kind = ctx.tcx.def_kind(super::id_to_def_id(*id));
            use rustc_hir::def::DefKind::*;
            match kind {
                Ctor(_, _) | Variant | Struct => true,
                _ => false,
            }
        }
    }
}

fn lower_value_path(ctx: &mut Ctx, path: Name) -> QName {
    if let Name::Path { id, .. } = path {
        let defid: DefId = super::id_to_def_id(id);
        crate::translation::translate_value_id(ctx.tcx, defid)
    } else {
        panic!("cannot lower a local identifier to a qualified name");
    }
}

fn lower_type_path(ctx: &mut Ctx, path: Name) -> QName {
    if let Name::Path { id, .. } = path {
        let defid: DefId = super::id_to_def_id(id);
        crate::ty::translate_ty_name(ctx, defid)
    } else {
        panic!("cannot lower a local identifier to a qualified name");
    }
}
