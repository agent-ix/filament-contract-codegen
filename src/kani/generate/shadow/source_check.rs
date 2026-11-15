//! The parsed-source checks of a `bounded_shadow` harness (FR-028-AC-13) and of the module that
//! holds its abstraction function (FR-028-AC-14).

use std::collections::{BTreeMap, BTreeSet};

use syn::{
    punctuated::Punctuated,
    visit::{self, Visit},
    Expr, ExprCall, ExprLit, ExprPath, ItemMod, Lit, Macro, Path, Token, UnOp,
};

/// Why a shadow harness source fails its check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShadowSourceViolation {
    /// The source does not parse.
    Unparsed(String),
    /// A path, `use` or `extern crate` names a crate outside `core`, `alloc`, `std` and `kani`.
    ForeignCrate(String),
    /// A `kani::stub` or `kani::stub_verified` attribute.
    Stub(String),
    /// A `kani::assume` that is not an argument-domain assumption.
    Assumption(String),
    /// A macro body that does not parse as expressions, so it cannot be checked.
    OpaqueMacro(String),
    /// The abstraction module is named by a path of the harness source.
    ImportsAbstraction(String),
}

/// One argument-domain assumption the source makes: the drawn name and its inclusive bounds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrawnBound {
    /// The drawn variable.
    pub argument: String,
    /// Inclusive minimum.
    pub minimum: i128,
    /// Inclusive maximum.
    pub maximum: i128,
}

const ALLOWED_CRATES: [&str; 4] = ["core", "alloc", "std", "kani"];
const PRIMITIVES: [&str; 17] = [
    "bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
    "u128", "usize", "f32", "f64",
];
/// The names the standard prelude brings into scope, which a path may start with.
const PRELUDE: [&str; 12] = [
    "Option", "Result", "Some", "None", "Ok", "Err", "Box", "Vec", "String", "Default", "From",
    "Into",
];
/// The only items of `kani` a shadow harness may name.
const KANI_ITEMS: [&str; 4] = ["proof", "any", "assume", "cover"];

struct Checker {
    local_modules: BTreeSet<String>,
    /// Types the file declares: a path may start with one.
    local_types: BTreeSet<String>,
    /// What each `use` binding names, so a bare `assume(..)` or `k::assume(..)` resolves.
    imports: BTreeMap<String, Vec<String>>,
    forbidden_module: Option<String>,
    violation: Option<ShadowSourceViolation>,
    assumptions: Vec<DrawnBound>,
}

impl Checker {
    fn fail(&mut self, violation: ShadowSourceViolation) {
        self.violation.get_or_insert(violation);
    }

    /// `segments` with a leading `use` binding replaced by what it names.
    fn expand(&self, segments: &[String]) -> Vec<String> {
        let mut expanded = segments.to_vec();
        for _ in 0..4 {
            let Some(target) = expanded.first().and_then(|first| self.imports.get(first)) else {
                break;
            };
            if target.first() == expanded.first() && target.len() == 1 {
                break;
            }
            let mut next = target.clone();
            next.extend(expanded.iter().skip(1).cloned());
            expanded = next;
        }
        expanded
    }

    fn check_path(&mut self, path: &Path) {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        self.check_segments(&segments, path.leading_colon.is_some());
    }

    fn check_segments(&mut self, written: &[String], leading_colon: bool) {
        let Some(first) = written.first() else {
            return;
        };
        if let Some(forbidden) = &self.forbidden_module {
            if written.iter().any(|segment| segment == forbidden) {
                self.fail(ShadowSourceViolation::ImportsAbstraction(
                    written.join("::"),
                ));
            }
        }
        let segments = self.expand(written);
        if segments.first().is_some_and(|name| name == "kani") {
            match segments.get(1) {
                None => {}
                Some(item) if KANI_ITEMS.contains(&item.as_str()) => {}
                Some(item) if item == "stub" || item == "stub_verified" => {
                    self.fail(ShadowSourceViolation::Stub(segments.join("::")));
                }
                Some(_) => {
                    self.fail(ShadowSourceViolation::ForeignCrate(segments.join("::")));
                }
            }
            return;
        }
        // A path written with a leading `::` always names an extern crate, whatever a local
        // module is called.
        if leading_colon {
            if !ALLOWED_CRATES.contains(&first.as_str()) {
                self.fail(ShadowSourceViolation::ForeignCrate(format!(
                    "::{}",
                    written.join("::")
                )));
            }
            return;
        }
        if written.len() < 2 {
            return;
        }
        let known = ALLOWED_CRATES.contains(&first.as_str())
            || matches!(first.as_str(), "self" | "super" | "Self")
            || PRIMITIVES.contains(&first.as_str())
            || self.local_modules.contains(first)
            || self.local_types.contains(first)
            || PRELUDE.contains(&first.as_str());
        if !known {
            self.fail(ShadowSourceViolation::ForeignCrate(written.join("::")));
        }
    }

    fn check_use_tree(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>, colon: bool) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.check_use_tree(&path.tree, prefix, colon);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                prefix.push(name.ident.to_string());
                self.check_use_root(prefix, colon);
                prefix.pop();
            }
            syn::UseTree::Rename(rename) => {
                prefix.push(rename.ident.to_string());
                self.check_use_root(prefix, colon);
                prefix.pop();
            }
            syn::UseTree::Glob(_) => {
                self.check_use_root(prefix, colon);
                if prefix.first().is_some_and(|name| name == "kani") {
                    self.fail(ShadowSourceViolation::ForeignCrate(format!(
                        "{}::*",
                        prefix.join("::")
                    )));
                }
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.check_use_tree(item, prefix, colon);
                }
            }
        }
    }

    fn check_use_root(&mut self, segments: &[String], colon: bool) {
        let Some(first) = segments.first() else {
            return;
        };
        if let Some(forbidden) = &self.forbidden_module {
            if segments.iter().any(|segment| segment == forbidden) {
                self.fail(ShadowSourceViolation::ImportsAbstraction(
                    segments.join("::"),
                ));
            }
        }
        if first == "kani" {
            self.check_segments(segments, false);
            return;
        }
        let local = !colon
            && (matches!(first.as_str(), "self" | "super") || self.local_modules.contains(first));
        if !(ALLOWED_CRATES.contains(&first.as_str()) || local) {
            self.fail(ShadowSourceViolation::ForeignCrate(segments.join("::")));
        }
    }

    /// The `cfg_attr` predicate and attributes, each attribute checked as if written alone.
    fn check_meta(&mut self, meta: &syn::Meta) {
        self.check_path(meta.path());
        if let syn::Meta::List(list) = meta {
            if list.path.is_ident("cfg_attr") {
                match list.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated) {
                    Ok(metas) => {
                        for inner in metas.iter().skip(1) {
                            self.check_meta(inner);
                        }
                    }
                    Err(_) => self.fail(ShadowSourceViolation::OpaqueMacro("cfg_attr".to_owned())),
                }
            }
        }
    }
}

fn integer_literal(expr: &Expr) -> Option<i128> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(value),
            ..
        }) => value.base10_parse::<i128>().ok(),
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => {
            integer_literal(&unary.expr).map(|value| -value)
        }
        Expr::Path(ExprPath { path, .. }) => {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            match segments.as_slice() {
                [kind, constant] if PRIMITIVES.contains(&kind.as_str()) => {
                    let width = match kind.as_str() {
                        "i64" => Some((i128::from(i64::MIN), i128::from(i64::MAX))),
                        _ => None,
                    }?;
                    match constant.as_str() {
                        "MIN" => Some(width.0),
                        "MAX" => Some(width.1),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        Expr::Paren(paren) => integer_literal(&paren.expr),
        _ => None,
    }
}

fn single_identifier(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(ExprPath { path, .. }) => path.get_ident().map(ToString::to_string),
        Expr::Paren(paren) => single_identifier(&paren.expr),
        _ => None,
    }
}

/// One side of an assumption as `(drawn name, minimum or maximum, which)`.
fn comparison(expr: &Expr) -> Option<(String, i128, bool)> {
    let Expr::Binary(binary) = expr else {
        return None;
    };
    // `x >= L` and `L <= x` bound from below; `x <= U` and `U >= x` bound from above.
    match &binary.op {
        syn::BinOp::Ge(_) => single_identifier(&binary.left)
            .zip(integer_literal(&binary.right))
            .map(|(name, bound)| (name, bound, true))
            .or_else(|| {
                single_identifier(&binary.right)
                    .zip(integer_literal(&binary.left))
                    .map(|(name, bound)| (name, bound, false))
            }),
        syn::BinOp::Le(_) => single_identifier(&binary.left)
            .zip(integer_literal(&binary.right))
            .map(|(name, bound)| (name, bound, false))
            .or_else(|| {
                single_identifier(&binary.right)
                    .zip(integer_literal(&binary.left))
                    .map(|(name, bound)| (name, bound, true))
            }),
        _ => None,
    }
}

fn domain_assumption(expr: &Expr) -> Option<DrawnBound> {
    let Expr::Binary(binary) = expr else {
        return None;
    };
    if !matches!(binary.op, syn::BinOp::And(_)) {
        return None;
    }
    let (first, first_is_lower) = comparison(&binary.left).map(|(n, b, l)| ((n, b), l))?;
    let (second, second_is_lower) = comparison(&binary.right).map(|(n, b, l)| ((n, b), l))?;
    if first.0 != second.0 || first_is_lower == second_is_lower {
        return None;
    }
    let (minimum, maximum) = if first_is_lower {
        (first.1, second.1)
    } else {
        (second.1, first.1)
    };
    Some(DrawnBound {
        argument: first.0,
        minimum,
        maximum,
    })
}

impl<'ast> Visit<'ast> for Checker {
    fn visit_path(&mut self, path: &'ast Path) {
        self.check_path(path);
        visit::visit_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.check_use_tree(&item.tree, &mut Vec::new(), item.leading_colon.is_some());
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        self.fail(ShadowSourceViolation::ForeignCrate(item.ident.to_string()));
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(ExprPath { path, .. }) = &*call.func {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            if self.expand(&segments) == ["kani", "assume"] {
                let admitted = call.args.len() == 1
                    && domain_assumption(&call.args[0])
                        .map(|bound| self.assumptions.push(bound))
                        .is_some();
                if !admitted {
                    self.fail(ShadowSourceViolation::Assumption(format!(
                        "an assume call with {} argument(s) that is not a domain bound",
                        call.args.len()
                    )));
                }
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        self.check_path(&node.path);
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        if matches!(
            name.as_str(),
            "assert" | "assert_eq" | "assert_ne" | "cover" | "debug_assert"
        ) {
            match node.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
                Ok(arguments) => {
                    for argument in &arguments {
                        self.visit_expr(argument);
                    }
                }
                Err(_) => self.fail(ShadowSourceViolation::OpaqueMacro(name)),
            }
        } else {
            self.fail(ShadowSourceViolation::OpaqueMacro(name));
        }
    }

    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        self.check_meta(&attribute.meta);
    }
}

#[derive(Default)]
struct Declarations {
    modules: BTreeSet<String>,
    types: BTreeSet<String>,
    imports: BTreeMap<String, Vec<String>>,
}

impl Declarations {
    fn import(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.import(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let mut target = prefix.clone();
                target.push(name.ident.to_string());
                self.imports.insert(name.ident.to_string(), target);
            }
            syn::UseTree::Rename(rename) => {
                let mut target = prefix.clone();
                target.push(rename.ident.to_string());
                self.imports.insert(rename.rename.to_string(), target);
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.import(item, prefix);
                }
            }
            syn::UseTree::Glob(_) => {}
        }
    }
}

impl<'ast> Visit<'ast> for Declarations {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        self.modules.insert(item.ident.to_string());
        visit::visit_item_mod(self, item);
    }
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.types.insert(item.ident.to_string());
        visit::visit_item_struct(self, item);
    }
    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.types.insert(item.ident.to_string());
        visit::visit_item_enum(self, item);
    }
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.types.insert(item.ident.to_string());
        visit::visit_item_trait(self, item);
    }
    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.types.insert(item.ident.to_string());
        visit::visit_item_type(self, item);
    }
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.import(&item.tree, &mut Vec::new());
    }
}

/// Parses `source` and returns the argument-domain bounds it assumes, or the first violation of
/// FR-028-AC-13: a crate outside `core`, `alloc`, `std` and `kani`, a `kani::stub`, or a
/// `kani::assume` that is not `x >= LOW && x <= HIGH` over one drawn name and two literals.
///
/// When `abstraction_module` is given, a path of `source` that names it is a violation of
/// FR-028-AC-14.
pub fn check_shadow_source(
    source: &str,
    abstraction_module: Option<&str>,
) -> Result<Vec<DrawnBound>, ShadowSourceViolation> {
    let file = syn::parse_file(source)
        .map_err(|error| ShadowSourceViolation::Unparsed(error.to_string()))?;
    let mut declarations = Declarations::default();
    declarations.visit_file(&file);
    let mut checker = Checker {
        local_modules: declarations.modules,
        local_types: declarations.types,
        imports: declarations.imports,
        forbidden_module: abstraction_module.map(ToOwned::to_owned),
        violation: None,
        assumptions: Vec::new(),
    };
    checker.visit_file(&file);
    match checker.violation {
        Some(violation) => Err(violation),
        None => Ok(checker.assumptions),
    }
}

/// Whether `source`, parsed, holds a loop construct (`while`, `loop`, `for`) or does not parse.
#[must_use]
pub fn contains_loop(source: &str) -> bool {
    struct Loops(bool);
    impl<'ast> Visit<'ast> for Loops {
        fn visit_expr_while(&mut self, _: &'ast syn::ExprWhile) {
            self.0 = true;
        }
        fn visit_expr_loop(&mut self, _: &'ast syn::ExprLoop) {
            self.0 = true;
        }
        fn visit_expr_for_loop(&mut self, _: &'ast syn::ExprForLoop) {
            self.0 = true;
        }
    }
    let Ok(file) = syn::parse_file(source) else {
        return true;
    };
    let mut loops = Loops(false);
    loops.visit_file(&file);
    loops.0
}

/// Why a shadow source is not straight-line code (FR-015-AC-72).
///
/// # Errors
///
/// The first loop, recursive call, heap type or integer type wider than `i64` that `source`,
/// parsed, holds; or that it does not parse.
pub fn check_straight_line(source: &str) -> Result<(), String> {
    const HEAP: [&str; 12] = [
        "Box",
        "Vec",
        "Rc",
        "Arc",
        "String",
        "HashMap",
        "BTreeMap",
        "HashSet",
        "BTreeSet",
        "VecDeque",
        "BinaryHeap",
        "Cow",
    ];
    const WIDE: [&str; 2] = ["i128", "u128"];
    struct Scan {
        problem: Option<String>,
        calls: BTreeMap<String, BTreeSet<String>>,
        current: Vec<String>,
    }
    impl<'ast> Visit<'ast> for Scan {
        fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
            self.current.push(function.sig.ident.to_string());
            visit::visit_item_fn(self, function);
            self.current.pop();
        }
        fn visit_expr_call(&mut self, call: &'ast ExprCall) {
            if let (Expr::Path(path), Some(caller)) = (&*call.func, self.current.last()) {
                if let Some(name) = path.path.get_ident() {
                    self.calls
                        .entry(caller.clone())
                        .or_default()
                        .insert(name.to_string());
                }
            }
            visit::visit_expr_call(self, call);
        }
        fn visit_expr_while(&mut self, _: &'ast syn::ExprWhile) {
            self.problem.get_or_insert("a `while` loop".to_owned());
        }
        fn visit_expr_loop(&mut self, _: &'ast syn::ExprLoop) {
            self.problem.get_or_insert("a `loop`".to_owned());
        }
        fn visit_expr_for_loop(&mut self, _: &'ast syn::ExprForLoop) {
            self.problem.get_or_insert("a `for` loop".to_owned());
        }
        fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
            let name = segment.ident.to_string();
            if HEAP.contains(&name.as_str()) {
                self.problem
                    .get_or_insert(format!("the heap type `{name}`"));
            }
            if WIDE.contains(&name.as_str()) {
                self.problem
                    .get_or_insert(format!("the integer type `{name}`, wider than `i64`"));
            }
            visit::visit_path_segment(self, segment);
        }
        fn visit_macro(&mut self, mac: &'ast Macro) {
            if let Some(name) = mac.path.get_ident() {
                if matches!(name.to_string().as_str(), "vec" | "format") {
                    self.problem
                        .get_or_insert(format!("the allocating macro `{name}!`"));
                }
            }
            visit::visit_macro(self, mac);
        }
    }
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut scan = Scan {
        problem: None,
        calls: BTreeMap::new(),
        current: Vec::new(),
    };
    scan.visit_file(&file);
    if let Some(problem) = scan.problem {
        return Err(problem);
    }
    // A function that reaches itself through the free functions it calls is recursive.
    for start in scan.calls.keys() {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&String> = scan
            .calls
            .get(start)
            .map(|callees| callees.iter().collect())
            .unwrap_or_default();
        while let Some(name) = stack.pop() {
            if name == start {
                return Err(format!("the recursive function `{start}`"));
            }
            if seen.insert(name) {
                if let Some(callees) = scan.calls.get(name) {
                    stack.extend(callees.iter());
                }
            }
        }
    }
    Ok(())
}
