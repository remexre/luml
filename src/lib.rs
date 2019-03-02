#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(grammar);

use itertools::join;
use std::{
    fmt::{Display, Formatter, Result as FmtResult, Write},
    str::FromStr,
    sync::Arc,
};

#[derive(Clone, Debug, Display)]
pub enum SExpr {
    Atom(Arc<str>),
    #[display(fmt = "({})", "join(_0, \" \")")]
    List(Vec<SExpr>),
}

impl SExpr {
    pub fn as_atom(&self) -> Result<Arc<str>, String> {
        match self {
            SExpr::Atom(a) => Ok(a.clone()),
            l => Err(format!("Expected an atom, got {}", l)),
        }
    }

    pub fn as_atom_headed_list(&self) -> Result<(Arc<str>, Vec<SExpr>), String> {
        let mut l = self.as_list()?;
        if l.is_empty() {
            return Err(format!("Expected a non-empty list, got {}", self));
        }

        let head = l.remove(0).as_atom()?;
        Ok((head, l))
    }

    pub fn as_list(&self) -> Result<Vec<SExpr>, String> {
        match self {
            SExpr::Atom(s) => Err(format!("Expected a list, got {}", s)),
            SExpr::List(l) => Ok(l.clone()),
        }
    }

    pub fn parse_many(s: &str) -> Result<Vec<SExpr>, String> {
        grammar::SExprsParser::new()
            .parse(s)
            .map_err(|e| e.to_string())
    }
}

impl FromStr for SExpr {
    type Err = String;
    fn from_str(s: &str) -> Result<SExpr, String> {
        grammar::SExprParser::new()
            .parse(s)
            .map_err(|e| e.to_string())
    }
}

#[derive(Debug)]
pub enum TopLevel {
    Class {
        name: Arc<str>,
        parents: Vec<Arc<str>>,
        ctors: Vec<Ctor>,
        dtor: Dtor,
        methods: Vec<Method>,
        props: Vec<Prop>,
    },
    Interface {
        name: Arc<str>,
        parents: Vec<Arc<str>>,
        methods: Vec<Method>,
    },
}

impl TopLevel {
    pub fn from_sexpr(s: SExpr) -> Result<TopLevel, String> {
        let (head, mut body) = s.as_atom_headed_list()?;
        if body.is_empty() {
            return Err(format!("Missing name in {}", s));
        }

        let name = body.remove(0).as_atom()?;
        let first_non_parent = body
            .iter()
            .position(|sexpr| sexpr.as_atom().is_err())
            .unwrap_or_else(|| body.len());
        let members = body.split_off(first_non_parent);
        let parents = body
            .into_iter()
            .map(|sexpr| sexpr.as_atom())
            .collect::<Result<Vec<_>, _>>()?;

        match head.as_ref() {
            "class" => {
                let mut ctors = Vec::new();
                let mut dtor = Dtor::default();
                let mut methods = Vec::new();
                let mut props = Vec::new();

                for sexpr in members {
                    let (head, body) = sexpr.as_atom_headed_list()?;
                    match head.as_ref() {
                        "ctor" => ctors.push(Ctor::from_sexpr(AccessModifier::Public, sexpr)?),
                        "fn" => methods.push(Method::from_sexpr(AccessModifier::Public, sexpr)?),
                        "prop" => props.push(Prop::from_sexpr(AccessModifier::Public, sexpr)?),

                        "private" => {
                            for sexpr in body {
                                match sexpr.as_atom_headed_list()?.0.as_ref() {
                                    "ctor" => ctors
                                        .push(Ctor::from_sexpr(AccessModifier::Private, sexpr)?),
                                    "fn" => methods
                                        .push(Method::from_sexpr(AccessModifier::Private, sexpr)?),
                                    "prop" => props
                                        .push(Prop::from_sexpr(AccessModifier::Private, sexpr)?),
                                    _ => return Err(format!("Expected a member, got {}", sexpr)),
                                }
                            }
                        }
                        "protected" => {
                            for sexpr in body {
                                match sexpr.as_atom_headed_list()?.0.as_ref() {
                                    "ctor" => ctors
                                        .push(Ctor::from_sexpr(AccessModifier::Protected, sexpr)?),
                                    "fn" => methods.push(Method::from_sexpr(
                                        AccessModifier::Protected,
                                        sexpr,
                                    )?),
                                    "prop" => props
                                        .push(Prop::from_sexpr(AccessModifier::Protected, sexpr)?),
                                    _ => return Err(format!("Expected a member, got {}", sexpr)),
                                }
                            }
                        }

                        _ => return Err(format!("Expected a member, got {}", sexpr)),
                    }
                }

                Ok(TopLevel::Class {
                    name,
                    parents,
                    ctors,
                    dtor,
                    methods,
                    props,
                })
            }
            "interface" => {
                let methods = members
                    .into_iter()
                    .map(|s| Method::from_sexpr(AccessModifier::Public, s))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(TopLevel::Interface {
                    name,
                    parents,
                    methods,
                })
            }
            _ => Err(format!("Expected a top-level expression, got {}", s)),
        }
    }

    pub fn name(&self) -> Arc<str> {
        match self {
            TopLevel::Class { name, .. } => name.clone(),
            TopLevel::Interface { name, .. } => name.clone(),
        }
    }

    pub fn uml_edges(&self, interfaces: &[Arc<str>]) -> Vec<String> {
        let (name, parents) = match self {
            TopLevel::Class {
                name,
                parents,
                ctors: _,
                dtor: _,
                methods: _,
                props: _,
            } => (name, parents),
            TopLevel::Interface {
                name,
                parents,
                methods: _,
            } => (name, parents),
        };

        parents
            .into_iter()
            .map(|parent| {
                let arrow = if interfaces.contains(parent) {
                    "empty"
                } else {
                    "normal"
                };
                format!("{} -> {} [arrowtail={}, dir=back];", parent, name, arrow)
            })
            .collect()
    }

    pub fn uml_node(&self) -> String {
        match self {
            TopLevel::Class {
                name,
                parents: _,
                ctors,
                dtor,
                methods,
                props,
            } => {
                let mut s = ctors
                    .iter()
                    .map(|ctor| format!("{} {}({})\n", ctor.modifier, name, join(&ctor.args, ", ")))
                    .collect::<String>();
                if dtor != &Dtor::default() {
                    write!(s, "{} ~{}()\n", dtor.modifier, name).unwrap();
                }
                s += &join(methods, "\n");
                format!("{{{}|{}|{}}}", name, join(props, "\n"), s)
            }
            TopLevel::Interface {
                name,
                parents: _,
                methods,
            } => format!(
                "{{&lt;&lt;interface&gt;&gt;\n{}||{}}}",
                name,
                join(methods, "\n")
            ),
        }
    }
}

#[derive(Debug, Display, PartialEq)]
pub enum AccessModifier {
    #[display(fmt = "+")]
    Public,
    #[display(fmt = "#")]
    Protected,
    #[display(fmt = "-")]
    Private,
}

impl Default for AccessModifier {
    fn default() -> AccessModifier {
        AccessModifier::Public
    }
}

#[derive(Debug)]
pub struct Ctor {
    modifier: AccessModifier,
    args: Vec<Arg>,
}

impl Ctor {
    pub fn from_sexpr(modifier: AccessModifier, s: SExpr) -> Result<Ctor, String> {
        let (head, mut body) = s.as_atom_headed_list()?;
        if head.as_ref() != "ctor" || body.len() != 1 {
            return Err(format!("Expected a constructor, got {}", s));
        }

        let args = body.pop().unwrap().as_list()?;
        let args = args
            .into_iter()
            .map(Arg::from_sexpr)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Ctor { modifier, args })
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Dtor {
    modifier: AccessModifier,
    kind: DtorKind,
}

#[derive(Debug, PartialEq)]
pub enum DtorKind {
    Custom,
    Default,
    Virtual,
    Deleted,
}

impl Default for DtorKind {
    fn default() -> DtorKind {
        DtorKind::Default
    }
}

#[derive(Debug, Display)]
#[display(fmt = "{} {}({}) -&gt; {}", modifier, name, "join(args, \", \")", ret)]
pub struct Method {
    modifier: AccessModifier,
    name: Arc<str>,
    args: Vec<Arg>,
    ret: Ty,
}

impl Method {
    pub fn from_sexpr(modifier: AccessModifier, s: SExpr) -> Result<Method, String> {
        let (head, mut body) = s.as_atom_headed_list()?;
        if head.as_ref() != "fn" || body.len() != 3 {
            return Err(format!("Expected a method, got {}", s));
        }

        let ret = body.pop().unwrap();
        let args = body.pop().unwrap().as_list()?;
        let name = body.pop().unwrap().as_atom()?;

        let args = args
            .into_iter()
            .map(Arg::from_sexpr)
            .collect::<Result<Vec<_>, _>>()?;
        let ret = Ty::from_sexpr(ret)?;

        Ok(Method {
            modifier,
            name,
            args,
            ret,
        })
    }
}

#[derive(Debug)]
pub struct Arg(Option<Arc<str>>, Ty);

impl Arg {
    pub fn from_sexpr(s: SExpr) -> Result<Arg, String> {
        match s.clone() {
            SExpr::Atom(_) => Ok(Arg(None, Ty::from_sexpr(s)?)),
            SExpr::List(mut l) => {
                if l.len() != 2 {
                    return Err(format!("Expected a named argument, got {}", s));
                }

                let ty = Ty::from_sexpr(l.pop().unwrap())?;
                let name = l.pop().unwrap().as_atom()?;
                let name = if name.as_ref() == "_" {
                    None
                } else {
                    Some(name)
                };
                Ok(Arg(name, ty))
            }
        }
    }
}

impl Display for Arg {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        if let Some(name) = self.0.as_ref() {
            write!(fmt, "{}: {}", name, self.1)
        } else {
            write!(fmt, "{}", self.1)
        }
    }
}

#[derive(Debug, Display)]
#[display(fmt = "{} {}: {}", modifier, name, ty)]
pub struct Prop {
    modifier: AccessModifier,
    name: Arc<str>,
    ty: Ty,
}

impl Prop {
    pub fn from_sexpr(modifier: AccessModifier, s: SExpr) -> Result<Prop, String> {
        let (head, mut body) = s.as_atom_headed_list()?;
        if head.as_ref() != "prop" || body.len() != 2 {
            return Err(format!("Expected a property, got {}", s));
        }

        let ty = Ty::from_sexpr(body.pop().unwrap())?;
        let name = body.pop().unwrap().as_atom()?;

        Ok(Prop { modifier, name, ty })
    }
}

#[derive(Debug, Display)]
pub enum Ty {
    #[display(fmt = "{}&lt;{}&gt;", "_0", "join(_1, \", \")")]
    App(Box<Ty>, Vec<Ty>),
    Base(Arc<str>),
}

impl Ty {
    pub fn from_sexpr(s: SExpr) -> Result<Ty, String> {
        match s {
            SExpr::Atom(s) => Ok(Ty::Base(s)),
            SExpr::List(mut l) => {
                if l.is_empty() {
                    return Err(format!("Expected a type, got ()"));
                }

                let head = Ty::from_sexpr(l.remove(0))?;
                let body = l
                    .into_iter()
                    .map(Ty::from_sexpr)
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Ty::App(Box::new(head), body))
            }
        }
    }
}
