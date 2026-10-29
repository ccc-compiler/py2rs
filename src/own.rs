//! Ownership inference and the aliasing check.
//!
//! Python shares mutable objects between names; the generated Rust copies,
//! moves or borrows them. The three are indistinguishable as long as no object
//! that is shared in Python is modified and then observed through another
//! name. This pass decides how every parameter is passed (`&T`, `&mut T`, `T`),
//! where values can be moved instead of cloned, and rejects programs whose
//! output would depend on sharing.

use std::collections::{HashMap, HashSet};

use crate::diag::{err, Error, Res};
use crate::ir::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// `T` for Copy types
    Copy,
    /// `&T`
    Ref,
    /// `&mut T`
    Mut,
    /// `T`, the callee keeps or rebinds it
    Owned,
}

#[derive(Default, Debug)]
pub struct Own {
    pub params: Vec<Vec<Mode>>,
    /// owned params that are only rebound (never stored or mutated): callers may clone
    pub owned_local: Vec<Vec<bool>>,
    pub self_mut: Vec<bool>,
    /// returns a copy of something reachable from a parameter or `self`
    pub ret_alias: Vec<bool>,
    /// Var expressions whose value is moved
    pub moves: HashSet<u32>,
    /// places moved out with `take` (overwritten before they are read again)
    pub takes: HashSet<u32>,
    /// variables that need `mut`
    pub mut_vars: Vec<HashSet<VarId>>,
    /// loop variables assigned in the body
    pub rebound: Vec<HashSet<VarId>>,
    /// for statements that iterate with `iter_mut`
    pub iter_mut: HashSet<u32>,
    /// variables live at the head of a loop (stmt id → vars)
    pub loop_live: HashMap<u32, HashSet<VarId>>,
    pub immutable: Vec<bool>,
    /// variables written through (loop / narrowed variables: need `&mut`)
    pub via: Vec<HashSet<VarId>>,
    /// locals that only ever refer to existing values: `&T` instead of a copy
    pub views: Vec<HashSet<VarId>>,
    /// variables live after each statement (stmt id)
    pub live_out: HashMap<u32, HashSet<VarId>>,
    /// narrowed values bound by value (`while let Some(mut node) = cur`)
    pub consumed: Vec<HashSet<VarId>>,
    /// locals that are a mutable reference into a container (`item = d[k]; item.n += 1`)
    pub mut_views: Vec<HashSet<VarId>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Index,
    Field(String),
    Tuple(usize),
    /// the value inside an Optional
    Some,
}

type Path = Vec<Step>;

#[derive(Clone, Debug)]
struct Mutation {
    /// the variable written in the source (a loop variable maps to `root`)
    via: VarId,
    root: VarId,
    path: Path,
    deep: bool,
    pos: u32,
    line: u32,
    /// the statement that modifies
    sid: u32,
}

#[derive(Clone, Debug)]
enum Dest {
    /// consumed right away
    Temp,
    /// bound to a local variable
    Local(VarId),
    /// stored in a container / object / owned parameter
    Escape,
    Return,
}

pub fn analyze(p: &Program) -> Res<Own> {
    let mut o = Own {
        params: p
            .funcs
            .iter()
            .map(|f| f.params.iter().map(|q| if f.vars[q.var].ty.is_copy() { Mode::Copy } else { Mode::Ref }).collect())
            .collect(),
        owned_local: p.funcs.iter().map(|f| vec![false; f.params.len()]).collect(),
        self_mut: vec![false; p.funcs.len()],
        ret_alias: vec![false; p.funcs.len()],
        mut_vars: vec![HashSet::new(); p.funcs.len()],
        rebound: vec![HashSet::new(); p.funcs.len()],
        via: vec![HashSet::new(); p.funcs.len()],
        views: vec![HashSet::new(); p.funcs.len()],
        consumed: vec![HashSet::new(); p.funcs.len()],
        mut_views: p.funcs.iter().map(mut_view_candidates).collect(),
        immutable: vec![false; p.classes.len()],
        ..Default::default()
    };
    class_immutability(p, &mut o);
    for f in &p.funcs {
        definite(f)?;
    }
    // parameter modes: iterate to a fixpoint (calls propagate &mut)
    loop {
        let mut changed = false;
        for fid in 0..p.funcs.len() {
            let f = &p.funcs[fid];
            if !f.dispatch.is_empty() {
                // a dispatcher passes its arguments on: the most demanding mode wins
                let rank = |m: Mode| match m {
                    Mode::Copy => 0,
                    Mode::Ref => 1,
                    Mode::Mut => 2,
                    Mode::Owned => 3,
                };
                for i in 0..f.params.len() {
                    let m = f.dispatch.iter().map(|(_, x)| o.params[*x][i]).max_by_key(|m| rank(*m)).unwrap();
                    let local = f.dispatch.iter().all(|(_, x)| o.params[*x][i] != Mode::Owned || o.owned_local[*x][i]);
                    let local = m == Mode::Owned && local;
                    if m != o.params[fid][i] || local != o.owned_local[fid][i] {
                        o.params[fid][i] = m;
                        o.owned_local[fid][i] = local;
                        changed = true;
                    }
                }
                let sm = f.dispatch.iter().any(|(_, x)| o.self_mut[*x]);
                let ra = f.dispatch.iter().any(|(_, x)| o.ret_alias[*x]);
                if sm != o.self_mut[fid] || ra != o.ret_alias[fid] {
                    o.self_mut[fid] = sm;
                    o.ret_alias[fid] = ra;
                    changed = true;
                }
                continue;
            }
            let (modes, self_m, ret_alias, self_escapes) = {
                let mut a = Fn::new(p, &o, fid);
                // modes are not final yet: errors are reported by the last pass
                let _ = a.collect();
                let mut modes = vec![];
                for (i, q) in f.params.iter().enumerate() {
                    // a generator keeps its arguments (its body runs later, on its own)
                    if f.is_gen && o.params[fid][i] != Mode::Copy {
                        modes.push((Mode::Owned, false));
                        continue;
                    }
                    // a `nonlocal` variable of a nested function is the caller's, by `&mut`
                    if nonlocal(f, i) {
                        modes.push((Mode::Mut, false));
                        continue;
                    }
                    if o.params[fid][i] == Mode::Copy {
                        modes.push((Mode::Copy, false));
                        continue;
                    }
                    let mutated = a.muts.iter().any(|m| m.root == q.var);
                    let escapes = a.escaped.contains(&q.var);
                    let rebound = a.assigned.get(&q.var).copied().unwrap_or(0) > 0;
                    let m = if escapes || rebound {
                        Mode::Owned
                    } else if mutated {
                        Mode::Mut
                    } else {
                        Mode::Ref
                    };
                    // a @shared object is changed through its handle
                    let shared = matches!(f.vars[q.var].ty, Ty::Class(c) if p.classes[c].shared);
                    let m = if shared && m == Mode::Mut { Mode::Ref } else { m };
                    let m = match q.force {
                        Some(Force::Own) => Mode::Owned,
                        Some(Force::Mut) if m == Mode::Ref => Mode::Mut,
                        _ => m,
                    };
                    modes.push((m, m == Mode::Owned && !escapes && !mutated));
                }
                let self_m = match &f.ext {
                    Some(x) => x.self_mut,
                    None => f.self_var.map(|sv| a.muts.iter().any(|m| m.root == sv) && !f.is_init).unwrap_or(false)
                        && !f.class.is_some_and(|c| p.classes[c].shared),
                };
                let esc = f.self_var.map(|sv| a.escaped.contains(&sv)).unwrap_or(false);
                let mv = a.accepted_mut_views();
                (modes, self_m, a.ret_alias, (esc, mv))
            };
            let (self_escapes, mv) = self_escapes;
            if mv != o.mut_views[fid] {
                o.mut_views[fid] = mv;
                changed = true;
            }
            if self_escapes && !f.class.is_some_and(|c| p.classes[c].shared) {
                return err(f.line, "`self` cannot be stored or returned")
                    .map_err(|e| e.hint("return a copy of the fields you need"));
            }
            for (i, (m, local)) in modes.into_iter().enumerate() {
                if m != o.params[fid][i] || local != o.owned_local[fid][i] {
                    o.params[fid][i] = m;
                    o.owned_local[fid][i] = local;
                    changed = true;
                }
            }
            if self_m != o.self_mut[fid] {
                o.self_mut[fid] = self_m;
                changed = true;
            }
            if ret_alias != o.ret_alias[fid] {
                o.ret_alias[fid] = ret_alias;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // annotations that the code contradicts
    for (fid, f) in p.funcs.iter().enumerate() {
        for (i, q) in f.params.iter().enumerate() {
            let m = o.params[fid][i];
            let n = &f.vars[q.var].name;
            match (q.force, m) {
                (Some(Force::Ref), Mode::Mut) => {
                    return err(f.line, format!("parameter `{}` is Ref[...] but {}() modifies it", n, f.name)).map_err(|e| e.hint("use Mut[...]"))
                }
                (Some(Force::Ref | Force::Mut), Mode::Owned) => {
                    return err(f.line, format!("parameter `{}` is borrowed but {}() keeps or rebinds it", n, f.name))
                        .map_err(|e| e.hint("use Own[...]"))
                }
                _ => {}
            }
        }
    }
    for fid in 0..p.funcs.len() {
        let (moves, muts, rebound, iter_mut, loop_live, via, takes) = {
            let mut a = Fn::new(p, &o, fid);
            a.collect()?;
            a.check()?;
            let via: HashSet<VarId> = a.muts.iter().map(|m| m.via).collect();
            let views = a.views();
            let takes = std::mem::take(&mut a.takes);
            let lo = std::mem::take(&mut a.live_out);
            let consumed = std::mem::take(&mut a.consumed);
            let (m, mu, r, i, l) = a.results();
            (m, mu, r, i, l, via, (takes, views, lo, consumed))
        };
        let (takes, views, lo, consumed) = takes;
        o.consumed[fid] = consumed;
        o.live_out.extend(lo);
        o.views[fid] = views;
        o.takes.extend(takes);
        o.via[fid] = via;
        o.moves.extend(moves);
        o.mut_vars[fid] = muts;
        o.rebound[fid] = rebound;
        o.iter_mut.extend(iter_mut);
        o.loop_live.extend(loop_live);
    }
    Ok(o)
}

/// A class is immutable when no field is ever assigned outside `__init__`
/// and all field types are immutable.
fn class_immutability(p: &Program, o: &mut Own) {
    let mut assigned: HashSet<ClassId> = HashSet::new();
    fn walk(p: &Program, f: &Func, s: &[Stmt], out: &mut HashSet<ClassId>) {
        for st in s {
            let mut targets: Vec<&Expr> = vec![];
            match &st.kind {
                StmtKind::Assign(t, _) => collect_places(t, &mut targets),
                StmtKind::AugAssign(t, _, _) => targets.push(t),
                _ => {}
            }
            for t in targets {
                let mut e = t;
                loop {
                    match &e.kind {
                        ExprKind::Field(b, _) => {
                            if let Ty::Class(c) = &b.ty {
                                let in_init = f.is_init && f.class == Some(*c)
                                    && matches!(b.kind, ExprKind::Var(v) if Some(v) == f.self_var);
                                if !in_init {
                                    out.insert(*c);
                                }
                            }
                            e = b;
                        }
                        ExprKind::Index(b, _) | ExprKind::TupleField(b, _) => e = b,
                        _ => break,
                    }
                }
            }
            match &st.kind {
                _ => {
                    for b in st.blocks() {
                        walk(p, f, b, out);
                    }
                }
            }
        }
    }
    for f in &p.funcs {
        walk(p, f, &f.body, &mut assigned);
    }
    for (i, c) in p.classes.iter().enumerate() {
        o.immutable[i] = c.frozen || !assigned.contains(&i);
    }
    let shared: Vec<bool> = p.classes.iter().map(|c| c.shared).collect();
    // field types; a value of a class may be any of its subclasses
    loop {
        let mut changed = false;
        for (i, c) in p.classes.iter().enumerate() {
            if shared[i] {
                continue;
            }
            let subs_mut = p.subtree(i).iter().any(|x| !o.immutable[*x]);
            if o.immutable[i] && (subs_mut || !c.fields.iter().all(|f| immutable_ty(&f.ty, &o.immutable))) {
                o.immutable[i] = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // a @shared object: a copy of the handle is the same object, as in Python
    for (i, sh) in shared.iter().enumerate() {
        if *sh {
            o.immutable[i] = true;
        }
    }
}

fn collect_places<'a>(t: &'a Target, out: &mut Vec<&'a Expr>) {
    match t {
        Target::Var(_) => {}
        Target::Place(e) => out.push(e),
        Target::Tuple(v) => v.iter().for_each(|t| collect_places(t, out)),
    }
}

pub fn immutable_ty(t: &Ty, classes: &[bool]) -> bool {
    match t {
        Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::None => true,
        Ty::Tuple(ts) => ts.iter().all(|t| immutable_ty(t, classes)),
        Ty::Class(c) => classes[*c],
        Ty::Optional(t) => matches!(**t, Ty::Class(_)) && immutable_ty(t, classes),
        _ => false,
    }
}

fn prefix(a: &Path, b: &Path) -> bool {
    a.len() <= b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

/// Is parameter i a `nonlocal` variable captured by a nested function?
pub fn nonlocal(f: &Func, i: usize) -> bool {
    i >= f.params.len() - f.captures.len() && f.params[i].force == Some(Force::Mut)
}

/// Methods of builtin types that modify their receiver.
fn mutating(name: &str) -> bool {
    crate::gen::mutating_builtin(name)
}

/// Per-function analysis.
struct Fn<'a> {
    p: &'a Program,
    o: &'a Own,
    fid: FuncId,
    f: &'a Func,
    /// pre-order position of each statement and the end of its subtree
    pos: HashMap<u32, (u32, u32)>,
    loops: Vec<(u32, u32)>,
    cur_pos: u32,
    cur_stmt: u32,
    muts: Vec<Mutation>,
    escaped: HashSet<VarId>,
    assigned: HashMap<VarId, u32>,
    ret_alias: bool,
    /// loop variable → (root, path) it borrows from
    origin: HashMap<VarId, (VarId, Path)>,
    /// variables that hold a copy of a value still reachable elsewhere
    alias_src: HashMap<VarId, Vec<(VarId, Path)>>,
    pending: Vec<(VarId, Path, Option<VarId>, u32, u32)>,
    /// a copy of a still-live value stored in a container: (source root, path, position, line)
    stores: Vec<(VarId, Path, u32, u32)>,
    live_out: HashMap<u32, HashSet<VarId>>,
    live_in: HashMap<u32, HashSet<VarId>>,
    /// narrowed value → (statement that binds it, first statement where it is bound / None for let-else)
    narrow_site: HashMap<VarId, (u32, Option<u32>)>,
    /// narrowed values that take the Optional's value (moved out)
    consumed: HashSet<VarId>,
    loop_live: HashMap<u32, HashSet<VarId>>,
    moves: HashSet<u32>,
    iter_mut: HashSet<u32>,
    takes: HashSet<u32>,
    /// variables used as owned values somewhere
    owned_seen: HashSet<VarId>,
    /// comprehensions whose result is only looked at
    temp_comps: HashSet<u32>,
    in_closure: u32,
    /// for loops: stmt id → (iterated root/path, loop vars)
    loop_iters: Vec<(u32, Option<(VarId, Path)>, Vec<VarId>, bool)>,
    errors: Vec<Error>,
}

impl<'a> Fn<'a> {
    fn new(p: &'a Program, o: &'a Own, fid: FuncId) -> Self {
        Fn {
            p,
            o,
            fid,
            f: &p.funcs[fid],
            pos: HashMap::new(),
            loops: vec![],
            cur_pos: 0,
            cur_stmt: 0,
            muts: vec![],
            escaped: HashSet::new(),
            assigned: HashMap::new(),
            ret_alias: false,
            origin: HashMap::new(),
            alias_src: HashMap::new(),
            pending: vec![],
            stores: vec![],
            live_out: HashMap::new(),
            live_in: HashMap::new(),
            narrow_site: HashMap::new(),
            consumed: HashSet::new(),
            loop_live: HashMap::new(),
            moves: HashSet::new(),
            iter_mut: HashSet::new(),
            takes: HashSet::new(),
            owned_seen: HashSet::new(),
            temp_comps: HashSet::new(),
            in_closure: 0,
            loop_iters: vec![],
            errors: vec![],
        }
    }

    fn imm(&self, t: &Ty) -> bool {
        immutable_ty(t, &self.o.immutable)
    }

    fn results(self) -> (HashSet<u32>, HashSet<VarId>, HashSet<VarId>, HashSet<u32>, HashMap<u32, HashSet<VarId>>) {
        // variables whose object is modified (or borrowed mutably); a @shared
        // object is modified through its handle
        let shared = |v: &VarId| matches!(self.f.vars[*v].ty, Ty::Class(c) if self.p.classes[c].shared);
        let mut muts: HashSet<VarId> = self.muts.iter().map(|m| m.root).filter(|v| !shared(v)).collect();
        let mut rebound = HashSet::new();
        for v in self.assigned.keys() {
            match self.f.vars[*v].kind {
                VarKind::Bound => {
                    rebound.insert(*v);
                }
                VarKind::Param => {
                    muts.insert(*v);
                }
                _ => {}
            }
        }
        (self.moves, muts, rebound, self.iter_mut, self.loop_live)
    }

    // ------------------------------------------------------------ positions

    fn number(&mut self, s: &[Stmt], n: &mut u32) {
        for st in s {
            *n += 1;
            let start = *n;
            match &st.kind {
                StmtKind::While(_, a, b) | StmtKind::For(_, _, a, b) => {
                    self.number(a, n);
                    self.loops.push((start, *n));
                    self.number(b, n);
                }
                _ => {
                    for b in st.blocks() {
                        self.number(b, n);
                    }
                }
            }
            self.pos.insert(st.id, (start, *n));
        }
    }

    /// Can something at `q` happen after something at `p`?
    fn after(&self, p: u32, q: u32) -> bool {
        q >= p || self.loops.iter().any(|&(a, b)| a <= p && p <= b && a <= q && q <= b)
    }

    // ------------------------------------------------------------- collect

    fn collect(&mut self) -> Res<()> {
        let mut n = 0;
        self.number(&self.f.body, &mut n);
        self.liveness();
        // a mutable reference local is the object it was taken from
        for (w, src) in mut_view_sources(self.f) {
            if self.o.mut_views[self.fid].contains(&w) && !self.origin.contains_key(&w) {
                if let Some((r, p)) = self.place(src) {
                    // `node = node.children[c]` keeps the first origin
                    if r != w {
                        self.origin.insert(w, (r, p));
                    }
                }
            }
        }
        let f = self.f;
        self.block(&f.body);
        if let Some(e) = self.errors.first() {
            return Err(e.clone());
        }
        Ok(())
    }

    fn block(&mut self, s: &[Stmt]) {
        for st in s {
            self.stmt(st);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        self.cur_pos = self.pos[&s.id].0;
        self.cur_stmt = s.id;
        match &s.kind {
            StmtKind::Expr(e) => self.expr(e),
            StmtKind::Assign(t, v) => {
                self.expr(v);
                self.target_exprs(t);
                self.assign(t, v, s);
            }
            StmtKind::AugAssign(t, op, v) => {
                self.expr(t);
                self.expr(v);
                if let Some((r, mut path)) = self.place(t) {
                    // `xs[i] += 1` modifies xs; `xs += ys` modifies xs itself
                    if !matches!(t.ty, Ty::List(_) | Ty::Set(_) | Ty::Dict(..)) || matches!(t.kind, ExprKind::Index(..) | ExprKind::Field(..)) {
                        path.pop();
                    }
                    self.mutate(r, path, false, s.line);
                }
                if matches!(t.ty, Ty::List(_)) && *op == BinOp::Add {
                    // elements of the right side are stored
                    self.store_elements(v, s.line);
                }
            }
            StmtKind::AugVar(v, _, e) => {
                self.expr(e);
                *self.assigned.entry(*v).or_default() += 1;
            }
            StmtKind::If(c, a, b) => {
                self.expr(c);
                self.note_region(c, s.id, a);
                self.block(a);
                self.block(b);
            }
            StmtKind::While(c, a, b) => {
                self.expr(c);
                self.note_region(c, s.id, a);
                self.block(a);
                self.block(b);
            }
            StmtKind::For(t, it, body, orelse) => {
                self.iter(it);
                let src = self.iter_source(it);
                let mut vars = vec![];
                t.vars(&mut vars);
                self.bind_origins(t, it);
                let before = self.muts.len();
                self.block(body);
                // loop variables that are modified → iter_mut
                let direct: Vec<VarId> = vars
                    .iter()
                    .copied()
                    .filter(|v| !self.f.vars[*v].ty.is_copy() && self.muts[before..].iter().any(|m| self.via_reaches(m.via, *v)))
                    .collect();
                if !direct.is_empty() {
                    self.iter_mut.insert(s.id);
                }
                self.loop_iters.push((s.id, src, vars, !direct.is_empty()));
                self.block(orelse);
            }
            StmtKind::Return(Some(e)) => {
                self.expr(e);
                self.owned(e, Dest::Return, s.line);
            }
            StmtKind::Raise(_, Some(e)) => self.expr(e),
            StmtKind::Assert(c, m) => {
                self.expr(c);
                if let Some(m) = m {
                    self.expr(m)
                }
            }
            StmtKind::LetSome(v, e, body) => {
                self.expr(e);
                self.narrow_site.insert(*v, (s.id, None));
                self.bind_narrow(*v, e);
                self.block(body);
            }
            StmtKind::Try(b, hs, e, f) => {
                self.block(b);
                for h in hs {
                    self.block(&h.body);
                }
                self.block(e);
                self.block(f);
            }
            StmtKind::Del(e) => {
                self.expr(e);
                if let ExprKind::Index(b, _) = &e.kind {
                    if let Some((r, p)) = self.place(b) {
                        self.mutate(r, p, false, s.line);
                    }
                }
            }
            _ => {}
        }
    }

    fn target_exprs(&mut self, t: &Target) {
        match t {
            Target::Var(_) => {}
            Target::Place(e) => e.children(&mut |c| self.expr(c)),
            Target::Tuple(v) => v.iter().for_each(|t| self.target_exprs(t)),
        }
    }

    fn assign(&mut self, t: &Target, v: &Expr, s: &Stmt) {
        match t {
            Target::Var(w) => {
                *self.assigned.entry(*w).or_default() += 1;
                self.alias_src.remove(w);
                self.owned(v, Dest::Local(*w), s.line);
            }
            Target::Place(pl) => {
                if let Some((r, mut path)) = self.place(pl) {
                    path.pop();
                    // in __init__, `self.f = ...` builds the object
                    if !(self.f.is_init && Some(r) == self.f.self_var && path.is_empty()) {
                        self.mutate(r, path, false, s.line);
                    }
                }
                self.owned(v, Dest::Escape, s.line);
            }
            Target::Tuple(ts) => {
                if let ExprKind::Tuple(es) = &v.kind {
                    for (t, e) in ts.iter().zip(es.iter()) {
                        self.assign(t, e, s);
                    }
                } else {
                    let mut vs = vec![];
                    t.vars(&mut vs);
                    for w in vs {
                        *self.assigned.entry(w).or_default() += 1;
                        self.alias_src.remove(&w);
                    }
                    let mut pls = vec![];
                    collect_places(t, &mut pls);
                    for pl in pls {
                        if let Some((r, mut path)) = self.place(pl) {
                            path.pop();
                            self.mutate(r, path, false, s.line);
                        }
                    }
                    self.owned(v, Dest::Escape, s.line);
                }
            }
        }
    }

    fn mutate(&mut self, root: VarId, path: Path, deep: bool, line: u32) {
        // through loop variables to what they borrow
        let via = root;
        let (root, path) = self.resolve(root, path);
        let pos = self.cur_pos;
        let sid = self.cur_stmt;
        self.muts.push(Mutation { via, root, path, deep, pos, line, sid });
    }

    /// Is `via` the variable `v` or a narrowed value borrowed from it?
    fn via_reaches(&self, via: VarId, v: VarId) -> bool {
        let mut x = via;
        loop {
            if x == v {
                return true;
            }
            match self.origin.get(&x) {
                Some((r, _)) if self.f.vars[x].kind == VarKind::Narrow => x = *r,
                _ => return false,
            }
        }
    }

    fn resolve(&self, root: VarId, path: Path) -> (VarId, Path) {
        match self.origin.get(&root) {
            Some((r, p)) => {
                let mut np = p.clone();
                np.extend(path);
                self.resolve(*r, np)
            }
            None => (root, path),
        }
    }

    /// Root variable and path of a place expression.
    fn place(&self, e: &Expr) -> Option<(VarId, Path)> {
        match &e.kind {
            ExprKind::Var(v) => Some((*v, vec![])),
            ExprKind::Index(b, _) => {
                let (r, mut p) = self.place(b)?;
                if matches!(b.ty, Ty::Str) {
                    return None;
                }
                p.push(Step::Index);
                Some((r, p))
            }
            ExprKind::Field(b, f) => {
                let (r, mut p) = self.place(b)?;
                p.push(Step::Field(f.clone()));
                Some((r, p))
            }
            ExprKind::TupleField(b, i) => {
                let (r, mut p) = self.place(b)?;
                p.push(Step::Tuple(*i));
                Some((r, p))
            }
            ExprKind::Builtin("dict.setdefault", a) => {
                let (r, mut p) = self.place(&a[0])?;
                p.push(Step::Index);
                Some((r, p))
            }
            _ => None,
        }
    }

    fn expr(&mut self, e: &Expr) {
        // a comprehension that is only printed / measured keeps nothing
        match &e.kind {
            ExprKind::Builtin("print" | "len" | "str" | "repr" | "sum" | "any" | "all", args) => {
                for a in args {
                    let a = match &a.kind {
                        ExprKind::Iter(it) => match &**it {
                            Iter::Over(x) | Iter::Gen(x) => x,
                            _ => continue,
                        },
                        _ => a,
                    };
                    if let ExprKind::Comp(_) = a.kind {
                        self.temp_comps.insert(a.id);
                    }
                }
            }
            ExprKind::FStr(parts) => {
                for p in parts {
                    if let FPart::Val(x @ Expr { kind: ExprKind::Comp(_), .. }, _, _) = p {
                        self.temp_comps.insert(x.id);
                    }
                }
            }
            _ => {}
        }
        match &e.kind {
            ExprKind::Comp(c) => {
                self.in_closure += 1;
                for g in &c.gens {
                    self.iter(&g.iter);
                    self.bind_origins(&g.target, &g.iter);
                    for c in &g.conds {
                        self.expr(c);
                    }
                }
                self.expr(&c.elt);
                if let Some(v) = &c.val {
                    self.expr(v);
                }
                let dest = if c.kind == CompKind::Gen || self.temp_comps.contains(&e.id) { Dest::Temp } else { Dest::Escape };
                self.owned(&c.elt, dest.clone(), e.line);
                if let Some(v) = &c.val {
                    self.owned(v, dest, e.line);
                }
                self.in_closure -= 1;
                return;
            }
            ExprKind::Lambda(_, b) => {
                self.in_closure += 1;
                self.expr(b);
                self.in_closure -= 1;
                return;
            }
            ExprKind::Iter(it) => {
                self.iter(it);
                return;
            }
            ExprKind::IsInst(x, d, Some(v)) if !self.p.classes[*d].subs.is_empty() => {
                // narrowed to a class with subclasses: a copy of the value
                if let Some((r, p)) = self.place(x) {
                    let (r, p) = self.resolve(r, p);
                    self.pending.push((r, p, Some(*v), self.cur_pos, e.line));
                }
            }
            ExprKind::IsSome(x, _, Some(v)) | ExprKind::IsInst(x, _, Some(v)) => self.bind_narrow(*v, x),
            ExprKind::SomeOf(x) => self.owned(x, Dest::Escape, e.line),
            _ => {}
        }
        e.children(&mut |c| self.expr(c));
        match &e.kind {
            ExprKind::Call(fid, args) | ExprKind::SuperInit(fid, args) => self.call(*fid, None, args, e),
            ExprKind::Method(r, fid, args) => self.call(*fid, Some(r), args, e),
            ExprKind::New(c, args) => {
                if let Some(init) = self.p.classes[*c].init {
                    self.call(init, None, args, e);
                } else {
                    for a in args {
                        self.owned(a, Dest::Escape, e.line);
                    }
                }
            }
            ExprKind::List(v) | ExprKind::Set(v) | ExprKind::Tuple(v) => {
                for a in v {
                    self.owned(a, Dest::Escape, e.line);
                }
            }
            ExprKind::Dict(v) => {
                for (k, a) in v {
                    self.owned(k, Dest::Escape, e.line);
                    self.owned(a, Dest::Escape, e.line);
                }
            }
            ExprKind::Bin(BinOp::Mul, a, _) if matches!(a.ty, Ty::List(_)) => {
                if let Ty::List(t) = &a.ty {
                    if !self.imm(t) {
                        self.errors.push(Error::new(e.line, "`list * n` repeats the same object n times")
                            .hint("build the list with a comprehension: [[0] * m for _ in range(n)]"));
                    }
                }
            }
            ExprKind::Builtin(name, args) => self.builtin(name, args, e),
            // `d[k]` of a defaultdict inserts the default
            ExprKind::Index(b, k) if matches!(b.ty, Ty::Dict(_, _, DK::Default)) && !self.p.present.contains(&e.id) => {
                if let Some((r, p)) = self.place(b) {
                    self.mutate(r, p, false, e.line);
                }
                self.owned(k, Dest::Escape, e.line);
            }
            _ => {}
        }
    }

    fn builtin(&mut self, name: &str, args: &[Expr], e: &Expr) {
        if matches!(name, "inline" | "inline_expr") {
            // raw Rust may modify what it names
            for a in &args[1..] {
                if let Some((r, p)) = self.place(a) {
                    if !a.ty.is_copy() {
                        self.mutate(r, p, true, e.line);
                    }
                }
            }
            return;
        }
        if mutating(name) {
            if let Some((r, p)) = self.place(&args[0]) {
                self.mutate(r, p, false, e.line);
            }
        }
        match name {
            "list.append" | "set.add" => self.owned(&args[1], Dest::Escape, e.line),
            "list.insert" => self.owned(&args[2], Dest::Escape, e.line),
            "dict.setdefault" => {
                self.owned(&args[1], Dest::Escape, e.line);
                self.owned(&args[2], Dest::Escape, e.line);
            }
            "list.extend" | "deque.extend" | "counter.update" => self.store_elements(&args[1], e.line),
            "yield" => self.owned(&args[0], Dest::Escape, e.line),
            "heapq.heappush" | "heapq.heappushpop" | "heapq.heapreplace" | "bisect.insort_left" | "bisect.insort_right"
            | "deque.append" | "deque.appendleft" => self.owned(&args[1], Dest::Escape, e.line),
            "dict.update" => self.store_elements(&args[1], e.line),
            _ => {}
        }
    }

    /// The elements of an iterable are stored somewhere else.
    fn store_elements(&mut self, it: &Expr, line: u32) {
        let elem = match &it.ty {
            Ty::List(t) | Ty::Set(t) => (**t).clone(),
            Ty::Dict(_, v, _) => (**v).clone(),
            t => t.clone(),
        };
        if self.imm(&elem) {
            return;
        }
        let inner = match &it.kind {
            ExprKind::Iter(i) => match &**i {
                Iter::Over(x) | Iter::Values(x) | Iter::Items(x) => Some(x),
                _ => None,
            },
            _ => Some(it),
        };
        if let Some(x) = inner {
            if let Some((r, _)) = self.place(x) {
                if self.movable_once(x) {
                    self.moves.insert(x.id);
                    return;
                }
                let _ = r;
                self.errors.push(Error::new(line, "the elements would be shared between two lists")
                    .hint("copy them: xs.extend([r.copy() for r in ys])"));
            }
        }
    }

    fn call(&mut self, fid: FuncId, recv: Option<&Expr>, args: &[Expr], e: &Expr) {
        let callee = &self.p.funcs[fid];
        if let Some(r) = recv {
            if self.o.self_mut[fid] {
                if let Some((root, p)) = self.place(r) {
                    self.mutate(root, p, true, e.line);
                }
            }
        }
        let mut mut_roots: Vec<VarId> = vec![];
        for (i, a) in args.iter().enumerate() {
            match self.o.params[fid][i] {
                Mode::Mut => {
                    if matches!(a.kind, ExprKind::Upcast(_)) {
                        self.errors.push(Error::new(e.line, format!(
                            "{}() modifies this argument, which is a subclass value passed as its base class",
                            callee.name)).hint("store it in a variable typed as the base class first"));
                    }
                    if let Some((root, p)) = self.place(a) {
                        let (rr, _) = self.resolve(root, vec![]);
                        mut_roots.push(rr);
                        self.mutate(root, p, true, e.line);
                    }
                }
                Mode::Owned => {
                    let d = if self.o.owned_local[fid][i] { Dest::Temp } else { Dest::Escape };
                    self.owned(a, d, e.line);
                }
                _ => {}
            }
        }
        // a value passed as &mut must not be passed again
        for (i, a) in args.iter().enumerate() {
            if self.o.params[fid][i] == Mode::Mut {
                continue;
            }
            let mut vs = vec![];
            uses(a, &mut vs);
            for v in vs {
                let (rr, _) = self.resolve(v, vec![]);
                if mut_roots.contains(&rr) && !a.ty.is_copy() {
                    self.errors.push(Error::new(e.line, format!(
                        "`{}` is passed to {}() twice while being modified",
                        self.f.vars[v].name, callee.name)));
                }
            }
        }
        // `self.state.insert(self)`: an argument modified (or a receiver modified)
        // while the other one borrows the same object
        if let Some(r) = recv {
            if let Some((root, rp)) = self.place(r) {
                let (rr, _) = self.resolve(root, rp.clone());
                for (i, a) in args.iter().enumerate() {
                    if self.o.params[fid][i] != Mode::Mut || rp.is_empty() {
                        continue;
                    }
                    let mut vs = vec![];
                    uses(a, &mut vs);
                    if vs.iter().any(|v| self.resolve(*v, vec![]).0 == rr) {
                        self.errors.push(Error::new(e.line, format!(
                            "{}() would modify `{}` while one of its parts is in use as the receiver",
                            callee.name, self.f.vars[root].name))
                            .hint("let the method return what should change, and apply it afterwards"));
                    }
                }
            }
        }
        if let Some(r) = recv {
            if self.o.self_mut[fid] {
                for a in args {
                    let mut vs = vec![];
                    uses(a, &mut vs);
                    if let Some((root, _)) = self.place(r) {
                        let (rr, _) = self.resolve(root, vec![]);
                        if vs.iter().any(|v| self.resolve(*v, vec![]).0 == rr) && !a.ty.is_copy() {
                            self.errors.push(Error::new(e.line, format!(
                                "`{}.{}()` modifies `{}` and also reads it in an argument",
                                self.f.vars[root].name, callee.name, self.f.vars[root].name)));
                        }
                    }
                }
            }
        }
    }

    /// The value of `e` is needed as an owned value going to `dest`.
    fn owned(&mut self, e: &Expr, dest: Dest, line: u32) {
        if e.ty.is_copy() {
            return;
        }
        if let ExprKind::IfExp(_, a, b) = &e.kind {
            self.owned(a, dest.clone(), line);
            self.owned(b, dest, line);
            return;
        }
        if let ExprKind::Upcast(x) = &e.kind {
            self.owned(x, dest, line);
            return;
        }
        // a whole variable that dies here is moved
        if let ExprKind::Var(v) = &e.kind {
            self.owned_seen.insert(*v);
            let var = &self.f.vars[*v];
            let owned_var = match var.kind {
                VarKind::Local => true,
                VarKind::Param => {
                    let i = self.f.params.iter().position(|q| q.var == *v).unwrap();
                    self.o.params[self.fid][i] == Mode::Owned
                }
                VarKind::Bound => !self.origin.contains_key(v),
                VarKind::SelfParam => false,
                VarKind::Narrow => self.consumable(*v),
            };
            if matches!(var.kind, VarKind::Param | VarKind::SelfParam) && matches!(dest, Dest::Escape | Dest::Return) {
                self.escaped.insert(*v);
            }
            if owned_var && self.movable_once(e) {
                self.moves.insert(e.id);
                if var.kind == VarKind::Narrow {
                    self.consumed.insert(*v);
                }
                // a moved alias carries its sources along
                if let Some(srcs) = self.alias_src.get(v).cloned() {
                    match dest {
                        Dest::Local(w) => {
                            for (r, p) in &srcs {
                                self.pending.push((*r, p.clone(), Some(w), self.cur_pos, line));
                            }
                            self.alias_src.insert(w, srcs);
                        }
                        Dest::Escape => self.errors.push(Error::new(line, format!(
                            "`{}` shares its value with another variable and cannot be stored",
                            var.name)).hint("store a copy: .copy()")),
                        _ => {}
                    }
                }
                return;
            }
        }
        if self.imm(&e.ty) {
            return;
        }
        // `node.next = self.head; self.head = node`: the old value moves
        if matches!(e.kind, ExprKind::Field(..) | ExprKind::Var(_))
            && matches!(e.ty, Ty::Optional(_) | Ty::List(_) | Ty::Dict(..) | Ty::Set(_) | Ty::Str)
            && self.in_closure == 0
            && self.overwritten_next(e)
        {
            self.takes.insert(e.id);
            if let Some((r, mut p)) = self.place(e) {
                p.pop();
                self.mutate(r, p, false, line);
            }
            return;
        }
        let sources = self.sources(e);
        if sources.is_empty() {
            return;
        }
        match dest {
            Dest::Temp => {}
            // a mutable reference local really is the same object
            Dest::Local(w) if self.o.mut_views[self.fid].contains(&w) => {}
            Dest::Local(w) => {
                for (r, p) in &sources {
                    self.pending.push((*r, p.clone(), Some(w), self.cur_pos, line));
                }
                self.alias_src.insert(w, sources);
            }
            Dest::Return => {
                for (r, _) in &sources {
                    if matches!(self.f.vars[*r].kind, VarKind::Param | VarKind::SelfParam) {
                        self.ret_alias = true;
                    }
                }
            }
            // stored as a copy; checked in `check` against later modifications
            Dest::Escape => {
                for (r, p) in sources {
                    self.stores.push((r, p, self.cur_pos, line));
                }
            }
        }
    }

    /// Is place `e` assigned again by a later statement of the same block before
    /// anything else reads it (and not used again in this statement)?
    fn overwritten_next(&self, e: &Expr) -> bool {
        let Some((root, path)) = self.place(e) else { return false };
        if self.f.vars[root].kind == VarKind::Bound || path.is_empty() && self.f.vars[root].kind != VarKind::SelfParam && self.f.vars[root].kind != VarKind::Param {
            return false;
        }
        let Some(cur) = self.find_stmt(self.cur_stmt) else { return false };
        // `self.head = self.head.next`: the target's old value is dropped here
        if let StmtKind::Assign(Target::Place(t), v) = &cur.kind {
            if let Some((tr, tp)) = self.place(t) {
                let (tr, tp) = self.resolve(tr, tp);
                let (er, ep) = self.resolve(root, path.clone());
                if tr == er && ep.len() > tp.len() && prefix(&tp, &ep) && std::ptr::eq(v, e) {
                    return true;
                }
            }
        }
        // the place occurs once in this statement
        let mut n = 0;
        let count = |x: &Expr, n: &mut i32| {
            fn go(x: &Expr, root: VarId, path: &Path, f: &Fn, n: &mut i32) {
                if let Some((r, p)) = f.place(x) {
                    if r == root && (prefix(&p, path) || prefix(path, &p)) && matches!(x.kind, ExprKind::Var(_) | ExprKind::Field(..)) && p.len() == path.len() {
                        *n += 1;
                        return;
                    }
                }
                x.children(&mut |c| go(c, root, path, f, n));
            }
            go(x, root, &path, self, n)
        };
        stmt_exprs_all(cur, &mut |x| count(x, &mut n));
        if n != 1 {
            return false;
        }
        let Some(after) = following(&self.f.body, self.cur_stmt) else { return false };
        for s in after {
            if let StmtKind::Assign(Target::Place(pl), v) = &s.kind {
                if self.place(pl) == Some((root, path.clone())) {
                    let mut u = vec![];
                    uses(v, &mut u);
                    return !u.iter().any(|w| self.resolve(*w, vec![]).0 == root);
                }
            }
            let mut u = vec![];
            stmt_exprs_all(s, &mut |x| uses(x, &mut u));
            if u.iter().any(|w| self.resolve(*w, vec![]).0 == root) || !s.blocks().is_empty() {
                return false;
            }
        }
        false
    }

    /// Locals assigned only from places of `self`, parameters or themselves,
    /// never modified: they can be references.
    fn views(&self) -> HashSet<VarId> {
        let mut assigns: HashMap<VarId, Vec<&Expr>> = HashMap::new();
        let mut bad: HashSet<VarId> = HashSet::new();
        fn walk<'b>(s: &'b [Stmt], a: &mut HashMap<VarId, Vec<&'b Expr>>, bad: &mut HashSet<VarId>) {
            for x in s {
                match &x.kind {
                    StmtKind::Assign(Target::Var(w), v) => a.entry(*w).or_default().push(v),
                    StmtKind::Assign(t, _) => {
                        let mut vs = vec![];
                        t.vars(&mut vs);
                        bad.extend(vs);
                    }
                    StmtKind::AugVar(w, ..) => {
                        bad.insert(*w);
                    }
                    _ => {}
                }
                for b in x.blocks() {
                    walk(b, a, bad);
                }
            }
        }
        walk(&self.f.body, &mut assigns, &mut bad);
        let mut out = HashSet::new();
        'var: for (w, vals) in assigns {
            let var = &self.f.vars[w];
            if var.kind != VarKind::Local || bad.contains(&w) || !matches!(var.ty, Ty::Optional(_) | Ty::Class(_) | Ty::Dict(..) | Ty::Tuple(_)) || var.ty.is_copy() {
                continue;
            }
            if self.muts.iter().any(|m| m.root == w || m.via == w) || self.o.mut_views[self.fid].contains(&w) {
                continue;
            }
            for v in vals {
                if !matches!(v.kind, ExprKind::Var(_) | ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::TupleField(..)) || self.takes.contains(&v.id) {
                    continue 'var;
                }
                let Some((r, p)) = self.place(v) else { continue 'var };
                let (r, _) = self.resolve(r, p);
                // a local source must keep its value while the reference is in use
                let local_ok = self.f.vars[r].kind == VarKind::Local && !self.assigned_while_live(r, w);
                let ok = r == w || local_ok || matches!(self.f.vars[r].kind, VarKind::Param | VarKind::SelfParam);
                if !ok {
                    continue 'var;
                }
                if let VarKind::Param = self.f.vars[r].kind {
                    let i = self.f.params.iter().position(|q| q.var == r).unwrap();
                    if self.o.params[self.fid][i] == Mode::Owned {
                        continue 'var;
                    }
                }
                // a write to what it borrows while it is still needed
                let (r2, p2) = self.resolve(r, self.place(v).map(|x| x.1).unwrap_or_default());
                for m in &self.muts {
                    if m.root != r2 || !(prefix(&m.path, &p2) || prefix(&p2, &m.path)) {
                        continue;
                    }
                    let live = self.live_out.get(&m.sid).map(|l| l.contains(&w)).unwrap_or(true);
                    let used_there = self.find_stmt(m.sid).map(|st| {
                        let mut u = vec![];
                        stmt_uses_shallow(st, &mut u);
                        u.contains(&w)
                    }).unwrap_or(true);
                    if live || used_there {
                        continue 'var;
                    }
                }
            }
            out.insert(w);
        }
        out
    }

    /// Candidates that are modified, never used as owned values, and whose
    /// container is not touched otherwise while they are in use.
    fn accepted_mut_views(&self) -> HashSet<VarId> {
        let mut out = HashSet::new();
        let srcs = mut_view_sources(self.f);
        for w in &self.o.mut_views[self.fid] {
            if self.owned_seen.contains(w) || !self.muts.iter().any(|m| m.via == *w) {
                continue;
            }
            // the borrowed part: the first source not taken from w itself
            let Some((_, src)) = srcs.iter().find(|(x, e)| x == w && self.place(e).map(|(r, _)| r != *w).unwrap_or(false)) else { continue };
            let Some((r, p)) = self.place(src) else { continue };
            if self.conflicts(*w, r, &p, &srcs) {
                continue;
            }
            out.insert(*w);
        }
        out
    }

    /// Is `r` (at a path overlapping `p`) used while `w` is in use, other than
    /// in w's own assignments?
    fn conflicts(&self, w: VarId, r: VarId, p: &Path, srcs: &[(VarId, &Expr)]) -> bool {
        let own_srcs: HashSet<u32> = srcs.iter().filter(|(x, _)| *x == w).map(|(_, e)| e.id).collect();
        fn places(f: &Fn, e: &Expr, r: VarId, skip: &HashSet<u32>, out: &mut Vec<Path>) {
            if skip.contains(&e.id) {
                return;
            }
            if let Some((root, path)) = f.place(e) {
                if root == r && matches!(e.kind, ExprKind::Var(_) | ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::TupleField(..)) {
                    out.push(path);
                    // index expressions inside may still use r
                    if let ExprKind::Index(_, i) = &e.kind {
                        places(f, i, r, skip, out);
                    }
                    return;
                }
            }
            e.children(&mut |c| places(f, c, r, skip, out));
        }
        fn walk(f: &Fn, s: &[Stmt], w: VarId, r: VarId, p: &Path, skip: &HashSet<u32>) -> bool {
            for st in s {
                let mut u = vec![];
                stmt_uses_shallow(st, &mut u);
                let active = u.contains(&w) || f.live_out.get(&st.id).map(|l| l.contains(&w)).unwrap_or(false);
                if active {
                    let mut ps = vec![];
                    stmt_exprs_all(st, &mut |e| places(f, e, r, skip, &mut ps));
                    if ps.iter().any(|q| prefix(q, p) || prefix(p, q)) {
                        return true;
                    }
                }
                if st.blocks().iter().any(|b| walk(f, b, w, r, p, skip)) {
                    return true;
                }
            }
            false
        }
        walk(self, &self.f.body, w, r, p, &own_srcs)
    }

    /// Is `r` assigned at a statement after which `w` is still needed?
    fn assigned_while_live(&self, r: VarId, w: VarId) -> bool {
        fn go(f: &Fn, s: &[Stmt], r: VarId, w: VarId) -> bool {
            s.iter().any(|st| {
                let hit = match &st.kind {
                    StmtKind::Assign(t, _) => {
                        let mut d = vec![];
                        t.vars(&mut d);
                        d.contains(&r)
                    }
                    StmtKind::AugVar(v, ..) => *v == r,
                    _ => false,
                };
                (hit && f.live_out.get(&st.id).map(|l| l.contains(&w)).unwrap_or(true)) || st.blocks().iter().any(|b| go(f, b, r, w))
            })
        }
        go(self, &self.f.body, r, w)
    }

    /// Where the value of a non-fresh expression lives.
    fn sources(&self, e: &Expr) -> Vec<(VarId, Path)> {
        let elem_mut = |t: &Ty| match t {
            Ty::List(x) | Ty::Set(x) => !self.imm(x),
            Ty::Dict(k, v, _) => !self.imm(k) || !self.imm(v),
            _ => true,
        };
        let mut out = vec![];
        match &e.kind {
            ExprKind::Var(_) | ExprKind::Index(..) | ExprKind::Field(..) | ExprKind::TupleField(..) => {
                if let Some((r, p)) = self.place(e) {
                    let (r, p) = self.resolve(r, p);
                    if let Some(srcs) = self.alias_src.get(&r) {
                        out.extend(srcs.iter().cloned());
                    }
                    out.push((r, p));
                }
            }
            ExprKind::Slice(b, ..) if elem_mut(&e.ty) => {
                for (r, mut p) in self.sources(b) {
                    p.push(Step::Index);
                    out.push((r, p));
                }
            }
            ExprKind::Builtin("list.copy" | "dict.copy" | "set.copy", a) if elem_mut(&e.ty) => {
                for (r, mut p) in self.sources(&a[0]) {
                    p.push(Step::Index);
                    out.push((r, p));
                }
            }
            ExprKind::Bin(BinOp::Add, a, b) if matches!(e.ty, Ty::List(_)) && elem_mut(&e.ty) => {
                for x in [a, b] {
                    for (r, mut p) in self.sources(x) {
                        p.push(Step::Index);
                        out.push((r, p));
                    }
                }
            }
            ExprKind::Builtin("dict.get", a) => {
                for (r, mut p) in self.sources(&a[0]) {
                    p.push(Step::Index);
                    out.push((r, p));
                }
            }
            ExprKind::Builtin("list" | "set" | "sorted" | "min_iter" | "max_iter", a) => {
                if let ExprKind::Iter(it) = &a[0].kind {
                    let elem = match &a[0].ty {
                        t => t.clone(),
                    };
                    if !self.imm(&elem) {
                        if let Some((r, mut p)) = self.iter_source(it) {
                            if !matches!(e.kind, ExprKind::Builtin("min_iter" | "max_iter", _)) || true {
                                p.push(Step::Index);
                            }
                            out.push((r, p));
                        }
                    }
                }
            }
            ExprKind::Call(fid, args) | ExprKind::Method(_, fid, args) if self.o.ret_alias[*fid] => {
                if let ExprKind::Method(r, _, _) = &e.kind {
                    out.extend(self.sources(r));
                }
                for a in args {
                    if !a.ty.is_copy() {
                        out.extend(self.sources(a));
                    }
                }
            }
            ExprKind::IfExp(_, a, b) => {
                out.extend(self.sources(a));
                out.extend(self.sources(b));
            }
            ExprKind::Upcast(x) => out.extend(self.sources(x)),
            _ => {}
        }
        out
    }

    fn iter_source(&self, it: &Iter) -> Option<(VarId, Path)> {
        match it {
            Iter::Over(x) | Iter::Keys(x) | Iter::Values(x) | Iter::Items(x) => {
                let (r, p) = self.place(x)?;
                Some(self.resolve(r, p))
            }
            Iter::Enumerate(i, _) | Iter::Reversed(i) => self.iter_source(i),
            _ => None,
        }
    }

    fn iter(&mut self, it: &Iter) {
        // iterating a generator object advances it
        if let Iter::Over(x) = it {
            if matches!(x.ty, Ty::Gen(_)) {
                if let Some((r, p)) = self.place(x) {
                    self.mutate(r, p, false, x.line);
                }
            }
        }
        match it {
            Iter::Gen(e) => self.expr(e),
            _ => it.exprs(&mut |x| self.expr(x)),
        }
    }

    fn note_region(&mut self, c: &Expr, sid: u32, body: &[Stmt]) {
        let mut vs = vec![];
        fn go(e: &Expr, out: &mut Vec<VarId>) {
            if let ExprKind::IsSome(_, true, Some(v)) = e.kind {
                out.push(v);
            }
            e.children(&mut |x| go(x, out));
        }
        go(c, &mut vs);
        for v in vs {
            self.narrow_site.insert(v, (sid, body.first().map(|b| b.id)));
        }
    }

    /// Can a narrowed value take ownership of the Optional local it comes from?
    /// Only if that local is not read again before it is assigned.
    fn consumable(&self, v: VarId) -> bool {
        let Some((r, p)) = self.origin.get(&v) else { return false };
        if p != &vec![Step::Some] || self.f.vars[*r].kind != VarKind::Local {
            return false;
        }
        match self.narrow_site.get(&v) {
            Some((_, Some(first))) => !self.live_in.get(first).map(|l| l.contains(r)).unwrap_or(true),
            Some((sid, None)) => !self.live_out.get(sid).map(|l| l.contains(r)).unwrap_or(true),
            _ => false,
        }
    }

    /// The value of a narrowed Optional borrows from its place.
    fn bind_narrow(&mut self, v: VarId, place: &Expr) {
        if let Some((r, mut p)) = self.place(place) {
            p.push(Step::Some);
            self.origin.insert(v, (r, p));
        }
    }

    /// Loop variables bound by reference borrow from the iterated value.
    fn bind_origins(&mut self, t: &Target, it: &Iter) {
        fn go(f: &mut Fn, t: &Target, it: &Iter) {
            match (t, it) {
                (_, Iter::Range(..)) | (_, Iter::Gen(_)) => {}
                (Target::Var(v), Iter::Over(x) | Iter::Values(x)) => {
                    if matches!(x.ty, Ty::Str) {
                        return;
                    }
                    if let Some((r, mut p)) = f.place(x) {
                        p.push(Step::Index);
                        f.origin.insert(*v, (r, p));
                    }
                }
                (Target::Var(_), Iter::Keys(_)) => {}
                (Target::Tuple(ts), Iter::Items(x)) if ts.len() == 2 => {
                    if let (Target::Var(v), Some((r, mut p))) = (&ts[1], f.place(x)) {
                        p.push(Step::Index);
                        f.origin.insert(*v, (r, p));
                    }
                }
                (Target::Tuple(ts), Iter::Enumerate(i, _)) if ts.len() == 2 => go(f, &ts[1], i),
                (Target::Tuple(ts), Iter::Zip(its)) if ts.len() == its.len() => {
                    for (t, i) in ts.iter().zip(its.iter()) {
                        go(f, t, i);
                    }
                }
                (Target::Tuple(ts), Iter::Over(x)) => {
                    // list of tuples: each component borrows from the element
                    if let Some((r, mut p)) = f.place(x) {
                        p.push(Step::Index);
                        for (i, t) in ts.iter().enumerate() {
                            if let Target::Var(v) = t {
                                let mut q = p.clone();
                                q.push(Step::Tuple(i));
                                f.origin.insert(*v, (r, q));
                            }
                        }
                    }
                }
                (t, Iter::Reversed(i)) => go(f, t, i),
                _ => {}
            }
        }
        go(self, t, it)
    }

    // ------------------------------------------------------------ liveness

    fn liveness(&mut self) {
        let body = &self.f.body;
        let mut ctx = vec![];
        self.live_block(body, HashSet::new(), &mut ctx);
    }

    fn live_block(&mut self, s: &[Stmt], out: HashSet<VarId>, ctx: &mut Vec<(HashSet<VarId>, HashSet<VarId>)>) -> HashSet<VarId> {
        let mut live = out;
        for st in s.iter().rev() {
            self.live_out.insert(st.id, live.clone());
            live = self.live_stmt(st, live, ctx);
            self.live_in.insert(st.id, live.clone());
        }
        live
    }

    fn live_stmt(&mut self, s: &Stmt, out: HashSet<VarId>, ctx: &mut Vec<(HashSet<VarId>, HashSet<VarId>)>) -> HashSet<VarId> {
        let mut u = vec![];
        match &s.kind {
            StmtKind::Expr(e) | StmtKind::Del(e) => {
                uses(e, &mut u);
                out.into_iter().chain(u).collect()
            }
            StmtKind::Assign(t, e) => {
                let mut d = vec![];
                t.vars(&mut d);
                let mut live: HashSet<VarId> = out.into_iter().filter(|v| !d.contains(v)).collect();
                uses(e, &mut u);
                target_uses(t, &mut u);
                live.extend(u);
                live
            }
            StmtKind::AugAssign(t, _, e) => {
                uses(t, &mut u);
                uses(e, &mut u);
                out.into_iter().chain(u).collect()
            }
            StmtKind::AugVar(v, _, e) => {
                uses(e, &mut u);
                u.push(*v);
                out.into_iter().chain(u).collect()
            }
            StmtKind::If(c, a, b) => {
                uses(c, &mut u);
                let bound = bound_in(c);
                let la: HashSet<VarId> = self.live_block(a, out.clone(), ctx).into_iter().filter(|v| !bound.contains(v)).collect();
                let lb = self.live_block(b, out, ctx);
                la.into_iter().chain(lb).chain(u).collect()
            }
            StmtKind::While(c, body, orelse) => {
                uses(c, &mut u);
                let le = self.live_block(orelse, out.clone(), ctx);
                let mut head: HashSet<VarId> = le.iter().copied().chain(u.iter().copied()).collect();
                // a narrowing condition binds its value for the body
                let bound = bound_in(c);
                loop {
                    ctx.push((out.clone(), head.clone()));
                    let lb = self.live_block(body, head.clone(), ctx);
                    ctx.pop();
                    let new: HashSet<VarId> = head.iter().copied().chain(lb.into_iter().filter(|v| !bound.contains(v))).collect();
                    if new == head {
                        break;
                    }
                    head = new;
                }
                self.loop_live.insert(s.id, head.clone());
                head
            }
            StmtKind::For(t, it, body, orelse) => {
                let mut d = vec![];
                t.vars(&mut d);
                it.exprs(&mut |e| uses(e, &mut u));
                let le = self.live_block(orelse, out.clone(), ctx);
                // the iterated value is borrowed for the whole loop
                let mut head: HashSet<VarId> = le.iter().copied().chain(u.iter().copied()).collect();
                loop {
                    ctx.push((out.clone(), head.clone()));
                    let lb = self.live_block(body, head.clone(), ctx);
                    ctx.pop();
                    let new: HashSet<VarId> =
                        head.iter().copied().chain(lb.into_iter().filter(|v| !d.contains(v))).collect();
                    if new == head {
                        break;
                    }
                    head = new;
                }
                self.loop_live.insert(s.id, head.clone());
                head
            }
            StmtKind::Return(e) => {
                if let Some(e) = e {
                    uses(e, &mut u);
                }
                u.into_iter().collect()
            }
            StmtKind::Raise(_, e) => {
                if let Some(e) = e {
                    uses(e, &mut u);
                }
                u.into_iter().collect()
            }
            StmtKind::Assert(c, m) => {
                uses(c, &mut u);
                if let Some(m) = m {
                    uses(m, &mut u);
                }
                out.into_iter().chain(u).collect()
            }
            StmtKind::LetSome(v, e, body) => {
                uses(e, &mut u);
                let lb = self.live_block(body, HashSet::new(), ctx);
                out.into_iter().filter(|x| x != v).chain(u).chain(lb).collect()
            }
            StmtKind::Try(b, hs, e, f) => {
                // any statement of the body may jump to a handler: what the
                // handlers and `finally` need stays alive through the body
                // an exception no handler takes runs finally and leaves: nothing after is read
                let lf_exc = self.live_block(f, HashSet::new(), ctx);
                let lf = self.live_block(f, out, ctx);
                let le = self.live_block(e, lf.clone(), ctx);
                let mut lh: HashSet<VarId> = lf_exc;
                for h in hs {
                    lh.extend(self.live_block(&h.body, lf.clone(), ctx));
                }
                let lb_out: HashSet<VarId> = le.iter().chain(lh.iter()).copied().collect();
                let lb = self.live_block(b, lb_out, ctx);
                fn add(f: &mut Fn, s: &[Stmt], extra: &HashSet<VarId>) {
                    for x in s {
                        if let Some(l) = f.live_out.get_mut(&x.id) {
                            l.extend(extra.iter().copied());
                        }
                        for blk in x.blocks() {
                            add(f, blk, extra);
                        }
                    }
                }
                add(self, b, &lh);
                lb.into_iter().chain(lh).collect()
            }
            StmtKind::Reraise(_) => HashSet::new(),
            StmtKind::Break => ctx.last().map(|c| c.0.clone()).unwrap_or_default(),
            StmtKind::Continue => ctx.last().map(|c| c.1.clone()).unwrap_or_default(),
            StmtKind::Pass => out,
        }
    }

    /// `e` (a variable) is used exactly once in the current statement, outside
    /// closures, and is dead afterwards.
    fn movable_once(&self, e: &Expr) -> bool {
        let ExprKind::Var(v) = e.kind else { return false };
        if self.in_closure > 0 {
            return false;
        }
        let s = self.find_stmt(self.cur_stmt);
        let Some(s) = s else { return false };
        let mut n = 0;
        let mut count = |x: &Expr| {
            let mut u = vec![];
            uses(x, &mut u);
            n += u.iter().filter(|w| **w == v).count();
        };
        match &s.kind {
            StmtKind::Expr(x) | StmtKind::Return(Some(x)) => count(x),
            // `d[key] = v`: the key (a copy) is evaluated before v moves into insert()
            StmtKind::Assign(Target::Place(Expr { kind: ExprKind::Index(b, _), .. }), x)
                if matches!(b.ty, Ty::Dict(..)) && matches!(x.kind, ExprKind::Var(w) if w == v) =>
            {
                count(x);
                let mut u = vec![];
                uses(b, &mut u);
                n += u.iter().filter(|w| **w == v).count();
            }
            StmtKind::Assign(t, x) => {
                count(x);
                let mut u = vec![];
                target_uses(t, &mut u);
                n += u.iter().filter(|w| **w == v).count();
            }
            StmtKind::AugAssign(t, _, x) => {
                count(t);
                count(x);
            }
            StmtKind::AugVar(w, _, x) => {
                count(x);
                if *w == v {
                    n += 1;
                }
            }
            _ => return false,
        }
        if n != 1 {
            return false;
        }
        if matches!(s.kind, StmtKind::Return(_)) {
            return true;
        }
        !self.live_out.get(&s.id).map(|l| l.contains(&v)).unwrap_or(true)
    }

    fn find_stmt(&self, id: u32) -> Option<&'a Stmt> {
        fn go(s: &[Stmt], id: u32) -> Option<&Stmt> {
            for st in s {
                if st.id == id {
                    return Some(st);
                }
                for b in st.blocks() {
                    if let Some(x) = go(b, id) {
                        return Some(x);
                    }
                }
            }
            None
        }
        go(&self.f.body, id)
    }

    // ---------------------------------------------------------------- check

    fn check(&mut self) -> Res<()> {
        // `for i in ...` over a local `i`: Python leaves the last item in `i`
        for (b, l) in &self.f.shadows {
            for (sid, _, vars, _) in &self.loop_iters {
                if vars.contains(b) && self.live_out.get(sid).map(|s| s.contains(l)).unwrap_or(false) {
                    let line = self.find_stmt(*sid).map(|s| s.line).unwrap_or(self.f.line);
                    let n = &self.f.vars[*l].name;
                    return Err(Error::new(line, format!("after this loop Python's `{}` holds the last item, not its earlier value", n))
                        .hint("use a different name for the loop variable"));
                }
            }
        }
        // copies must not observe later modifications
        for (root, path, dest, pos, line) in self.pending.clone() {
            for m in &self.muts {
                if !self.after(pos, m.pos) {
                    continue;
                }
                // the copy only differs from the original if it is used afterwards
                let dest_live = match dest {
                    Some(d) => self.live_out.get(&m.sid).map(|l| l.contains(&d)).unwrap_or(true),
                    None => true,
                };
                let hit_src = m.root == root && (prefix(&path, &m.path) || (m.deep && prefix(&m.path, &path))) && dest_live;
                let hit_dst = Some(m.root) == dest;
                if hit_src || hit_dst {
                    let src = &self.f.vars[root].name;
                    let what = match dest {
                        Some(d) => format!("`{}` and `{}`", self.f.vars[d].name, src),
                        None => format!("`{}`", src),
                    };
                    return Err(Error::new(line, format!(
                        "{} share one object in Python, and it is modified later (line {})", what, m.line))
                        .hint("make an explicit copy (.copy(), list(...), [r.copy() for r in ...]) or avoid the second name"));
                }
            }
        }
        // a stored copy must stay equal to its source while the source is used:
        // neither may be modified (anything reached through a path or modified
        // deeply might be the stored copy)
        for (root, path, pos, line) in self.stores.clone() {
            for m in &self.muts {
                if !self.after(pos, m.pos) {
                    continue;
                }
                let src_live = self.live_out.get(&m.sid).map(|l| l.contains(&root)).unwrap_or(true);
                let hit = if m.root == root {
                    prefix(&path, &m.path) || (m.deep && prefix(&m.path, &path))
                } else {
                    src_live && (m.deep || !m.path.is_empty())
                };
                if hit {
                    let src = &self.f.vars[root].name;
                    return Err(Error::new(line, format!(
                        "this stores `{}`, which Python shares with the container, and one of them is modified later (line {})",
                        src, m.line))
                        .hint("store a copy (.copy(), list(...), a new object)"));
                }
            }
        }
        // iterating a value while modifying it
        for (sid, src, vars, by_mut) in self.loop_iters.clone() {
            let Some((root, path)) = src else { continue };
            let (start, end) = self.pos[&sid];
            for m in &self.muts {
                if m.pos <= start || m.pos > end {
                    continue;
                }
                if m.root == root && (prefix(&path, &m.path) || prefix(&m.path, &path)) {
                    // modifications through the loop variable itself are fine
                    let through_var = by_mut && m.path.len() > path.len();
                    if through_var && vars.iter().any(|v| self.origin.contains_key(v)) {
                        continue;
                    }
                    if m.path.len() > path.len() && !by_mut {
                        // element modified through an index while iterating
                    }
                    return Err(Error::new(m.line, format!(
                        "`{}` is modified while a loop iterates over it", self.f.vars[root].name))
                        .hint("iterate over a copy (`for x in list(xs):`) or loop over indices"));
                }
            }
            if by_mut {
                // nothing else may touch the container while elements are borrowed mutably
                let body_uses = self.uses_in_range(start, end, root, &vars);
                if let Some(line) = body_uses {
                    return Err(Error::new(line, format!(
                        "`{}` is used inside a loop that modifies its elements", self.f.vars[root].name))
                        .hint("loop over indices: for i in range(len(xs)): ..."));
                }
            }
        }
        Ok(())
    }

    /// Line of a use of `root` (other than through `vars`) inside a loop body.
    fn uses_in_range(&self, start: u32, end: u32, root: VarId, vars: &[VarId]) -> Option<u32> {
        let mut found = None;
        let mut walk = |s: &Stmt, f: &Fn| {
            let (p, _) = f.pos[&s.id];
            if p <= start || p > end || found.is_some() {
                return;
            }
            let mut u = vec![];
            stmt_uses_shallow(s, &mut u);
            for v in u {
                if vars.contains(&v) || vars.iter().any(|lv| f.via_reaches(v, *lv)) {
                    continue;
                }
                if f.resolve(v, vec![]).0 == root {
                    found = Some(s.line);
                }
            }
        };
        fn all<'b>(s: &'b [Stmt], f: &Fn, w: &mut dyn FnMut(&Stmt, &Fn)) {
            for st in s {
                w(st, f);
                for b in st.blocks() {
                    all(b, f, w);
                }
            }
        }
        all(&self.f.body, self, &mut walk);
        found
    }
}

/// Variables used by an expression (including inside closures).
pub fn uses(e: &Expr, out: &mut Vec<VarId>) {
    if let ExprKind::Var(v) = e.kind {
        out.push(v);
    }
    e.children(&mut |c| uses(c, out));
}

fn target_uses(t: &Target, out: &mut Vec<VarId>) {
    match t {
        Target::Var(_) => {}
        Target::Place(e) => uses(e, out),
        Target::Tuple(v) => v.iter().for_each(|t| target_uses(t, out)),
    }
}

/// Uses in the statement's own expressions (not nested statements).
fn stmt_uses_shallow(s: &Stmt, out: &mut Vec<VarId>) {
    match &s.kind {
        StmtKind::Expr(e) | StmtKind::Del(e) | StmtKind::Return(Some(e)) | StmtKind::Raise(_, Some(e)) => uses(e, out),
        StmtKind::Assign(t, e) => {
            uses(e, out);
            target_uses(t, out);
        }
        StmtKind::AugAssign(t, _, e) => {
            uses(t, out);
            uses(e, out);
        }
        StmtKind::AugVar(v, _, e) => {
            out.push(*v);
            uses(e, out);
        }
        StmtKind::If(c, ..) | StmtKind::While(c, ..) | StmtKind::LetSome(_, c, _) => uses(c, out),
        StmtKind::For(_, it, ..) => it.exprs(&mut |e| uses(e, out)),
        StmtKind::Assert(c, m) => {
            uses(c, out);
            if let Some(m) = m {
                uses(m, out);
            }
        }
        _ => {}
    }
}

/// The statements after `id` in its block.
fn following(s: &[Stmt], id: u32) -> Option<&[Stmt]> {
    if let Some(i) = s.iter().position(|x| x.id == id) {
        return Some(&s[i + 1..]);
    }
    for x in s {
        for b in x.blocks() {
            if let Some(r) = following(b, id) {
                return Some(r);
            }
        }
    }
    None
}

/// All expressions of a statement (not nested statements), targets included.
fn stmt_exprs_all(s: &Stmt, f: &mut dyn FnMut(&Expr)) {
    match &s.kind {
        StmtKind::Expr(e) | StmtKind::Del(e) | StmtKind::Return(Some(e)) | StmtKind::Raise(_, Some(e)) => f(e),
        StmtKind::Assign(t, e) => {
            f(e);
            let mut ps = vec![];
            collect_places(t, &mut ps);
            ps.into_iter().for_each(|p| f(p));
        }
        StmtKind::AugAssign(t, _, e) => {
            f(t);
            f(e)
        }
        StmtKind::AugVar(_, _, e) => f(e),
        StmtKind::If(c, ..) | StmtKind::While(c, ..) | StmtKind::LetSome(_, c, _) => f(c),
        StmtKind::For(_, it, ..) => it.exprs(f),
        StmtKind::Assert(c, m) => {
            f(c);
            if let Some(m) = m {
                f(m)
            }
        }
        _ => {}
    }
}

// ------------------------------------------------------- definite assignment

/// Every read of a local must follow an assignment on every path (rustc
/// requires it; Python would raise UnboundLocalError on the other paths).
pub fn definite(f: &Func) -> Res<()> {
    // None: this point is not reachable
    type St = Option<HashSet<VarId>>;
    fn meet(a: St, b: St) -> St {
        match (a, b) {
            (None, x) | (x, None) => x,
            (Some(a), Some(b)) => Some(a.intersection(&b).copied().collect()),
        }
    }
    struct D<'a> {
        f: &'a Func,
        /// states at the breaks of each enclosing loop
        breaks: Vec<St>,
    }
    impl D<'_> {
        fn read(&self, e: &Expr, s: &HashSet<VarId>) -> Res<()> {
            let mut u = vec![];
            uses(e, &mut u);
            for v in u {
                if self.f.vars[v].kind == VarKind::Local && !s.contains(&v) {
                    return Err(Error::new(e.line, format!("`{}` might be used before it is assigned", self.f.vars[v].name))
                        .hint("assign it on every path before this point (Python would raise UnboundLocalError)"));
                }
            }
            Ok(())
        }
        fn block(&mut self, b: &[Stmt], s: St) -> Res<St> {
            let mut s = s;
            for st in b {
                s = self.stmt(st, s)?;
            }
            Ok(s)
        }
        fn stmt(&mut self, st: &Stmt, s: St) -> Res<St> {
            let Some(mut cur) = s else { return Ok(None) };
            match &st.kind {
                StmtKind::Assign(t, e) => {
                    self.read(e, &cur)?;
                    let mut r = Ok(());
                    t.exprs(&mut |x| {
                        if r.is_ok() {
                            r = self.read(x, &cur);
                        }
                    });
                    if let Target::Place(p) = t {
                        r = r.and_then(|_| self.read(p, &cur));
                    }
                    r?;
                    let mut d = vec![];
                    t.vars(&mut d);
                    cur.extend(d);
                    Ok(Some(cur))
                }
                StmtKind::AugVar(v, _, e) => {
                    self.read(e, &cur)?;
                    let line = st.line;
                    if self.f.vars[*v].kind == VarKind::Local && !cur.contains(v) {
                        return Err(Error::new(line, format!("`{}` might be used before it is assigned", self.f.vars[*v].name)));
                    }
                    Ok(Some(cur))
                }
                StmtKind::Expr(e) | StmtKind::Del(e) => {
                    self.read(e, &cur)?;
                    Ok(Some(cur))
                }
                StmtKind::AugAssign(t, _, e) => {
                    self.read(t, &cur)?;
                    self.read(e, &cur)?;
                    Ok(Some(cur))
                }
                StmtKind::Assert(c, m) => {
                    self.read(c, &cur)?;
                    if let Some(m) = m {
                        self.read(m, &cur)?;
                    }
                    Ok(Some(cur))
                }
                StmtKind::Return(e) | StmtKind::Raise(_, e) => {
                    if let Some(e) = e {
                        self.read(e, &cur)?;
                    }
                    Ok(None)
                }
                StmtKind::Reraise(_) | StmtKind::Continue => Ok(None),
                StmtKind::Break => {
                    if let Some(b) = self.breaks.last_mut() {
                        *b = meet(b.take(), Some(cur));
                    }
                    Ok(None)
                }
                StmtKind::Pass => Ok(Some(cur)),
                StmtKind::If(c, a, b) => {
                    self.read(c, &cur)?;
                    let x = self.block(a, Some(cur.clone()))?;
                    let y = self.block(b, Some(cur))?;
                    Ok(meet(x, y))
                }
                StmtKind::LetSome(_, e, b) => {
                    self.read(e, &cur)?;
                    self.block(b, Some(cur.clone()))?;
                    Ok(Some(cur))
                }
                StmtKind::While(c, body, orelse) => {
                    self.read(c, &cur)?;
                    self.breaks.push(None);
                    self.block(body, Some(cur.clone()))?;
                    let br = self.breaks.pop().unwrap();
                    let forever = matches!(c.kind, ExprKind::Bool(true));
                    let normal = if forever { None } else { self.block(orelse, Some(cur))? };
                    Ok(meet(normal, br))
                }
                StmtKind::For(_, it, body, orelse) => {
                    let mut r = Ok(());
                    it.exprs(&mut |x| {
                        if r.is_ok() {
                            r = self.read(x, &cur);
                        }
                    });
                    r?;
                    self.breaks.push(None);
                    self.block(body, Some(cur.clone()))?;
                    let br = self.breaks.pop().unwrap();
                    let normal = self.block(orelse, Some(cur))?;
                    Ok(meet(normal, br))
                }
                StmtKind::Try(b, hs, e, fin) => {
                    let ok = self.block(b, Some(cur.clone()))?;
                    let ok = self.block(e, ok)?;
                    let mut out = ok;
                    for h in hs {
                        // an exception can come before any assignment of the body
                        let x = self.block(&h.body, Some(cur.clone()))?;
                        out = meet(out, x);
                    }
                    if fin.is_empty() {
                        return Ok(out);
                    }
                    // finally runs on every path; it may only rely on what was assigned before the try
                    self.block(fin, Some(cur))?;
                    match out {
                        None => Ok(None),
                        Some(o) => {
                            let after = self.block(fin, Some(o))?;
                            Ok(after)
                        }
                    }
                }
            }
        }
    }
    let mut start = HashSet::new();
    for (i, v) in f.vars.iter().enumerate() {
        if v.kind != VarKind::Local {
            start.insert(i);
        }
    }
    let mut d = D { f, breaks: vec![] };
    d.block(&f.body, Some(start))?;
    Ok(())
}

/// Variables bound by narrowing tests in a condition.
fn bound_in(c: &Expr) -> Vec<VarId> {
    let mut out = vec![];
    fn go(e: &Expr, out: &mut Vec<VarId>) {
        if let ExprKind::IsSome(_, _, Some(v)) | ExprKind::IsInst(_, _, Some(v)) = e.kind {
            out.push(v);
        }
        e.children(&mut |x| go(x, out));
    }
    go(c, &mut out);
    out
}

/// Locals assigned only from places (never computed values): may become
/// mutable references.
fn mut_view_candidates(f: &Func) -> HashSet<VarId> {
    let srcs = mut_view_sources(f);
    let mut bad: HashSet<VarId> = HashSet::new();
    fn walk(s: &[Stmt], bad: &mut HashSet<VarId>) {
        for x in s {
            match &x.kind {
                StmtKind::Assign(Target::Var(w), v) => {
                    if !matches!(v.kind, ExprKind::Var(_) | ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::TupleField(..)) {
                        bad.insert(*w);
                    }
                }
                StmtKind::Assign(t, _) => {
                    let mut d = vec![];
                    t.vars(&mut d);
                    bad.extend(d);
                }
                StmtKind::AugVar(w, ..) => {
                    bad.insert(*w);
                }
                _ => {}
            }
            for b in x.blocks() {
                walk(b, bad);
            }
        }
    }
    walk(&f.body, &mut bad);
    let mut out = HashSet::new();
    for (w, _) in &srcs {
        let v = &f.vars[*w];
        if v.kind == VarKind::Local && !bad.contains(w) && matches!(v.ty, Ty::Class(_) | Ty::List(_) | Ty::Dict(..) | Ty::Set(_)) {
            out.insert(*w);
        }
    }
    out
}

/// (variable, assigned place) for every plain assignment of a local.
fn mut_view_sources(f: &Func) -> Vec<(VarId, &Expr)> {
    let mut out = vec![];
    fn walk<'b>(s: &'b [Stmt], out: &mut Vec<(VarId, &'b Expr)>) {
        for x in s {
            if let StmtKind::Assign(Target::Var(w), v) = &x.kind {
                out.push((*w, v));
            }
            for b in x.blocks() {
                walk(b, out);
            }
        }
    }
    walk(&f.body, &mut out);
    out
}

