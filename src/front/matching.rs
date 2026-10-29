//! `match` becomes an `if` / `elif` chain over the subject, written as Python and
//! lowered like hand-written code (so `isinstance` narrows as usual):
//!
//! ```text
//! match s:                                 if isinstance(s, Circle) and s.r > 0:
//!     case Circle(r=r) if r > 0: ...    →      r = s.r; ...
//!     case [x, *rest]: ...                 elif len(s) >= 1:
//!                                              x = s[0]; rest = s[1:]; ...
//! ```

use ruff_python_ast::relocate::relocate_expr;
use ruff_python_ast::visitor::transformer::{walk_expr, Transformer};
use ruff_text_size::TextRange;

use super::*;

/// The conditions and bindings of one pattern, as Python text.
#[derive(Default)]
struct Case {
    conds: Vec<String>,
    binds: Vec<(String, String)>,
}

/// Replaces bound names in a guard by the places they are bound to.
struct Subst<'a> {
    binds: &'a [(String, AExpr)],
}

impl Transformer for Subst<'_> {
    fn visit_expr(&self, e: &mut AExpr) {
        if let AExpr::Name(n) = e {
            if let Some((_, x)) = self.binds.iter().find(|(b, _)| b == n.id.as_str()) {
                *e = x.clone();
                return;
            }
        }
        walk_expr(self, e);
    }
}

impl Front {
    fn src_of(&self, r: &impl Ranged) -> String {
        format!("({})", &self.src[r.range()])
    }

    fn parse_at(&self, text: &str, at: TextRange, line: u32) -> Res<AExpr> {
        let mut e = ruff_python_parser::parse_expression(text)
            .map_err(|_| Error::new(line, format!("internal: match lowering produced `{}`", text)))?
            .into_expr();
        relocate_expr(&mut e, at);
        Ok(e)
    }

    fn pattern(&self, p: &ast::Pattern, place: &str, out: &mut Case, line: u32) -> Res<()> {
        use ast::Pattern as P;
        match p {
            P::MatchValue(v) => out.conds.push(format!("{} == {}", place, self.src_of(&*v.value))),
            P::MatchSingleton(s) => out.conds.push(match s.value {
                ast::Singleton::None => format!("{} is None", place),
                ast::Singleton::True => format!("{} == True", place),
                ast::Singleton::False => format!("{} == False", place),
            }),
            P::MatchAs(a) => {
                if let Some(p) = &a.pattern {
                    self.pattern(p, place, out, line)?;
                }
                if let Some(n) = &a.name {
                    out.binds.push((n.id.to_string(), place.to_string()));
                }
            }
            P::MatchOr(o) => {
                let mut alts = vec![];
                for p in &o.patterns {
                    let mut c = Case::default();
                    self.pattern(p, place, &mut c, line)?;
                    if !c.binds.is_empty() {
                        return err(line, "names bound inside `|` patterns are not supported");
                    }
                    if c.conds.is_empty() {
                        alts.clear();
                        break;
                    }
                    alts.push(format!("({})", c.conds.join(" and ")));
                }
                if !alts.is_empty() {
                    out.conds.push(format!("({})", alts.join(" or ")));
                }
            }
            P::MatchSequence(s) => {
                let star = s.patterns.iter().position(|p| matches!(p, P::MatchStar(_)));
                let n = s.patterns.len();
                match star {
                    None => out.conds.push(format!("len({}) == {}", place, n)),
                    Some(_) => out.conds.push(format!("len({}) >= {}", place, n - 1)),
                }
                for (i, p) in s.patterns.iter().enumerate() {
                    match (star, p) {
                        (_, P::MatchStar(st)) => {
                            if let Some(name) = &st.name {
                                let after = n - 1 - i;
                                let slice = if after == 0 {
                                    format!("{}[{}:]", place, i)
                                } else {
                                    format!("{}[{}:len({}) - {}]", place, i, place, after)
                                };
                                out.binds.push((name.id.to_string(), slice));
                            }
                        }
                        (Some(k), _) if i > k => self.pattern(p, &format!("{}[{}]", place, i as i64 - n as i64), out, line)?,
                        _ => self.pattern(p, &format!("{}[{}]", place, i), out, line)?,
                    }
                }
            }
            P::MatchClass(c) => {
                let AExpr::Name(cn) = &*c.cls else {
                    return err(line, "class patterns need a class name");
                };
                let Some(&cid) = self.classes.get(cn.id.as_str()) else {
                    return err(line, format!("`{}` is not a class of this program", cn.id))
                        .map_err(|e| e.hint("patterns of builtin types (int(), str()) are not supported"));
                };
                out.conds.push(format!("isinstance({}, {})", place, cn.id));
                let cl = &self.p.classes[cid];
                if !c.arguments.patterns.is_empty() {
                    if !cl.dataclass {
                        return err(line, format!("positional patterns need a dataclass; `{}` is not one", cl.name))
                            .map_err(|e| e.hint("use keyword patterns: C(field=x)"));
                    }
                    if c.arguments.patterns.len() > cl.fields.len() {
                        return err(line, format!("`{}` has {} fields", cl.name, cl.fields.len()));
                    }
                }
                let fields: Vec<String> = cl.fields.iter().map(|f| f.name.clone()).collect();
                for (p, f) in c.arguments.patterns.iter().zip(fields) {
                    self.pattern(p, &format!("{}.{}", place, f), out, line)?;
                }
                for k in &c.arguments.keywords {
                    self.pattern(&k.pattern, &format!("{}.{}", place, k.attr.id), out, line)?;
                }
            }
            P::MatchStar(_) => return err(line, "`*` outside a sequence pattern"),
            P::MatchMapping(_) => return err(line, "mapping patterns are not supported"),
        }
        Ok(())
    }

    /// The `if` chain for a `match` statement (with an assignment of the subject first
    /// when it is not a plain name).
    pub(super) fn lower_match(&mut self, m: &ast::StmtMatch, line: u32) -> Res<Vec<AStmt>> {
        let mut out = vec![];
        let at = m.range;
        // sequence patterns do not match strings in Python
        let subj = self.expr(&m.subject, None)?;
        if self.norm(&subj.ty) == Ty::Str
            && m.cases.iter().any(|c| matches!(c.pattern, ast::Pattern::MatchSequence(_)))
        {
            return err(line, "sequence patterns never match a str");
        }
        let place = match &*m.subject {
            AExpr::Name(n) => n.id.to_string(),
            _ => {
                self.next_id += 1;
                let name = format!("match_{}", self.next_id);
                let target = self.parse_at(&name, at, line)?;
                out.push(AStmt::Assign(ast::StmtAssign {
                    node_index: Default::default(),
                    range: at,
                    targets: vec![target],
                    value: m.subject.clone(),
                }));
                name
            }
        };
        let mut chain: Vec<(Option<AExpr>, Vec<AStmt>)> = vec![];
        for c in &m.cases {
            let cl = self.line(c);
            let mut k = Case::default();
            self.pattern(&c.pattern, &place, &mut k, cl)?;
            let crange = c.range;
            let binds: Vec<(String, AExpr)> =
                k.binds.iter().map(|(n, p)| Ok((n.clone(), self.parse_at(p, crange, cl)?))).collect::<Res<_>>()?;
            let mut conds: Vec<AExpr> = k.conds.iter().map(|t| self.parse_at(t, crange, cl)).collect::<Res<_>>()?;
            if let Some(g) = &c.guard {
                let mut g = (**g).clone();
                Subst { binds: &binds }.visit_expr(&mut g);
                conds.push(g);
            }
            let test = match conds.len() {
                0 => None,
                1 => conds.pop(),
                _ => Some(AExpr::BoolOp(ast::ExprBoolOp {
                    node_index: Default::default(),
                    range: crange,
                    op: ast::BoolOp::And,
                    values: conds,
                })),
            };
            let mut body = vec![];
            for (n, x) in binds {
                if n == "_" {
                    continue;
                }
                let target = self.parse_at(&n, crange, cl)?;
                body.push(AStmt::Assign(ast::StmtAssign {
                    node_index: Default::default(),
                    range: crange,
                    targets: vec![target],
                    value: Box::new(x),
                }));
            }
            body.extend(c.body.iter().cloned());
            let last = test.is_none();
            chain.push((test, body));
            if last {
                // an irrefutable case: later cases never run
                break;
            }
        }
        let mut it = chain.into_iter();
        let Some((first_test, first_body)) = it.next() else { return Ok(out) };
        let Some(test) = first_test else {
            out.extend(first_body);
            return Ok(out);
        };
        let clauses = it
            .map(|(t, b)| ast::ElifElseClause { node_index: Default::default(), range: at, test: t, body: b.into() })
            .collect();
        out.push(AStmt::If(ast::StmtIf {
            node_index: Default::default(),
            range: at,
            test: Box::new(test),
            body: first_body.into(),
            elif_else_clauses: clauses,
        }));
        Ok(out)
    }
}
