use std::collections::{BTreeSet, LinkedList};
use std::convert::{TryFrom, TryInto};
use std::fmt;

use failure::{bail, format_err, Fallible};
use proc_macro2::TokenStream;

pub fn reprs<'a>(attrs: impl 'a + Iterator<Item = &'a syn::Attribute>) -> Vec<TokenStream> {
    attrs
        .filter_map(|attr| {
            if !attr.path.is_ident("repr") {
                return None;
            }

            Some(attr.tts.clone())
        })
        .collect()
}

pub struct RenameRule(serde_derive_internals::attr::RenameRule);

impl RenameRule {
    pub fn apply_to_variant(&self, s: &str) -> String {
        self.0.apply_to_variant(s)
    }
}

impl fmt::Debug for RenameRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RenameRule")
            .finish()
    }
}

pub type ErrorList = LinkedList<failure::Error>;

macro_rules! bail_list {
    ($msg:literal $( , $args:expr )* $(,)?) => {
        {
            let mut list = ErrorList::new();
            list.push_back(failure::format_err!($msg, $($args),*));
            return Err(list);
        }
    }
}

#[derive(Debug)]
pub enum Attr {
    CaseInsensitive,
    Skip,
    Rename(String),
    RenameAll(RenameRule),
    Alias(String),
}

impl Attr {
    pub fn parse_attrs(attr: &syn::Attribute) -> impl Iterator<Item = Fallible<Self>> {
        use syn::NestedMeta;

        Self::get_args(attr)
            .map(|arg| {
                match arg {
                    NestedMeta::Meta(m) => Attr::try_from(&m),
                    _ => bail!("Argument to attribute cannot be a literal"),
                }
            })
    }

    /// Returns an iterator over the items in `...` if this attribute looks like `#[enumeration(...)]`
    fn get_args(attr: &syn::Attribute) -> impl Iterator<Item = syn::NestedMeta> {
        use syn::{Meta, MetaList};

        match attr.parse_meta() {
            Ok(Meta::List(MetaList { ident, nested, .. })) => if ident == "enumeration" {
                return nested.into_iter();
            }

            _ => {}
        }

        syn::punctuated::Punctuated::new().into_iter()
    }
}

/// Parse an attr from the `syn::Meta` inside parens after "enumeration".
impl TryFrom<&'_ syn::Meta> for Attr {
    type Error = failure::Error;

    fn try_from(meta: &syn::Meta) -> Result<Self, Self::Error> {
        use syn::{Lit, Meta, MetaNameValue};

        // Extracts a string literal from a MetaNameValue
        let lit_val = |lit: &syn::Lit| {
            match lit {
                Lit::Str(v) => Ok(v.value()),
                _ => bail!("Non-string literal"),
            }
        };

        match meta {
            // #[enumeration(skip)]
            Meta::Word(ident) if ident == "skip" =>
                Ok(Attr::Skip),

            // #[enumeration(case_insensitive)]
            Meta::Word(ident) if ident == "case_insensitive" =>
                Ok(Attr::CaseInsensitive),

            // #[enumeration(rename = "...")]
            Meta::NameValue(MetaNameValue { ident, lit, .. }) if ident == "rename" =>
                Ok(Attr::Rename(lit_val(lit)?)),

            // #[enumeration(rename_all = "...")]
            Meta::NameValue(MetaNameValue { ident, lit, .. }) if ident == "rename_all" => {
                let rule = lit_val(lit)?.parse().map_err(|_| format_err!("Invalid RenameAll rule"))?;
                Ok(Attr::RenameAll(RenameRule(rule)))
            }

            // #[enumeration(alias = "...")]
            Meta::NameValue(MetaNameValue { ident, lit, .. }) if ident == "alias" =>
                Ok(Attr::Alias(lit_val(lit)?)),

            _ => bail!("Unknown attribute argument")
        }
    }
}

#[derive(Debug, Default)]
pub struct VariantAttrs {
    pub skip: bool,
    pub rename: Option<String>,
    pub aliases: BTreeSet<String>,
}

impl VariantAttrs {
    pub fn from_attrs<T>(attrs: T) -> Result<Self, ErrorList>
        where T: IntoIterator<Item = Fallible<Attr>>,
    {
        let mut ret = VariantAttrs::default();
        let mut errors = ErrorList::default();
        for attr in attrs {
            match attr {
                Ok(Attr::Skip) => ret.skip = true,

                Ok(Attr::Rename(s)) => if ret.rename.is_none() {
                    ret.rename = Some(s);
                } else {
                    errors.push_back(format_err!("Variant cannot be renamed multiple times"));
                },

                Ok(Attr::Alias(s)) => {
                    ret.aliases.insert(s);
                },

                Ok(attr) =>
                    errors.push_back(format_err!("Attribute \"{:?}\" is not valid for a variant", attr)),

                Err(e) => errors.push_back(e),
            }
        }

        if errors.is_empty() {
            Ok(ret)
        } else {
            Err(errors)
        }
    }
}

#[derive(Default)]
pub struct EnumAttrs {
    pub nocase: bool,
    pub rename_rule: Option<RenameRule>,
}

impl EnumAttrs {
    pub fn from_attrs<T>(attrs: T) -> Result<Self, ErrorList>
        where T: IntoIterator<Item = Fallible<Attr>>,
    {
        let mut ret = EnumAttrs::default();
        let mut errors = ErrorList::default();
        for attr in attrs {
            match attr {
                Ok(Attr::CaseInsensitive) => ret.nocase = true,

                Ok(Attr::RenameAll(r)) => if ret.rename_rule.is_none() {
                    ret.rename_rule = Some(r);
                } else {
                    errors.push_back(format_err!("Enum can only have a single \"rename_all\" attribute"));
                },

                Ok(attr) =>
                    errors.push_back(format_err!("Attribute \"{:?}\" is not valid for an enum", attr)),

                Err(e) => errors.push_back(e),
            }
        }

        if errors.is_empty() {
            Ok(ret)
        } else {
            Err(errors)
        }
    }
}

pub type Discriminant = i128;

pub struct Enum<'a> {
    pub name: &'a syn::Ident,
    pub attrs: EnumAttrs,

    pub variants: Vec<(&'a syn::Variant, VariantAttrs)>,

    /// None if the enum is not C-like
    pub discriminants: Option<Vec<Discriminant>>,
}

impl<'a> Enum<'a> {
    pub fn parse(input: &'a syn::DeriveInput) -> Result<Self, ErrorList> {
        use syn::{Data, DataEnum, Expr, ExprLit, Lit};

        let DataEnum { variants, .. } = match &input.data {
            Data::Enum(e) => e,
            _ => bail_list!("Input must be an enum"),
        };

        let mut errors = ErrorList::default();
        let enum_attrs = EnumAttrs::from_attrs(input.attrs
            .iter()
            .flat_map(Attr::parse_attrs));

        let enum_attrs = match enum_attrs {
            Ok(attrs) => attrs,
            Err(e) => {
                errors = e;
                Default::default()
            }
        };

        let mut discriminants = Some(vec![]);
        let mut parsed_variants = vec![];
        for v in variants.iter() {
            let attrs = VariantAttrs::from_attrs(v.attrs
                .iter()
                .flat_map(Attr::parse_attrs));

            let attrs = match attrs {
                Ok(a) => a,
                Err(mut e) => {
                    errors.append(&mut e);
                    continue;
                }
            };

            parsed_variants.push((v, attrs));

            if v.fields != syn::Fields::Unit {
                discriminants = None;
                continue;
            }

            if let Some(ds) = discriminants.as_mut() {
                match &v.discriminant {
                    // An integer literal
                    Some((_, Expr::Lit(ExprLit { lit: Lit::Int(i), .. }))) => {
                        ds.push(i.value().try_into().expect("Variant overflowed i128"));
                    }

                    // An expr with an unknown value (e.g. a const defined elsewhere)
                    Some(_) => {
                        discriminants = None;
                    }

                    // No discriminant
                    None => {
                        let d = ds.last().map(|&x| x + 1).unwrap_or(0);
                        ds.push(d);
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Enum {
            name: &input.ident,
            attrs: enum_attrs,
            variants: parsed_variants,
            discriminants,
        })
    }
}
