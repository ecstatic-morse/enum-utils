use std::collections::BTreeMap;

use failure::format_err;
use proc_macro2::TokenStream;
use quote::quote;

use crate::attr::{Enum, ErrorList};
use enum_utils_from_str::{Case, StrMapFunc};

struct FromStrImpl {
    nocase: bool,
    enum_name: syn::Ident,
    variants: BTreeMap<String, syn::Ident>,
}

impl FromStrImpl {
    pub fn parse(input: &syn::DeriveInput) -> Result<Self, ErrorList> {
        let Enum { name, attrs: enum_attrs, variants, .. } = Enum::parse(input)?;

        let mut errors = ErrorList::default();
        let mut name_map = BTreeMap::default();
        for (v, attrs) in variants.iter() {
            if attrs.skip {
                continue;
            }

            if v.fields != syn::Fields::Unit {
                errors.push_back(format_err!("An (unskipped) variant cannot have fields"));
            }

            if let Some(name) = &attrs.rename {
                name_map.insert(name.clone(), v.ident.clone());
            } else if let Some(rename_rule) = &enum_attrs.rename_rule {
                let s = v.ident.to_string();
                name_map.insert(rename_rule.apply_to_variant(&*s), v.ident.clone());
            } else {
                let s = v.ident.to_string();
                name_map.insert(s, v.ident.clone());
            }

            for alias in &attrs.aliases {
                name_map.insert(alias.clone(), v.ident.clone());
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(FromStrImpl {
            nocase: enum_attrs.nocase,
            enum_name: name.clone(),
            variants: name_map,
        })
    }
}

pub fn derive(ast: &syn::DeriveInput) -> Result<TokenStream, ErrorList> {
    let FromStrImpl { nocase, enum_name, variants } = FromStrImpl::parse(ast)?;

    let mut trie = StrMapFunc::new("_parse", &enum_name.to_string());
    let case = if nocase { Case::Insensitive } else { Case::Sensitive };
    trie.case(case);

    for (alias, variant) in variants {
        let path = quote!(#enum_name::#variant);
        trie.entry(alias.as_str(), path);
    }

    Ok(quote!{
        impl ::std::str::FromStr for #enum_name {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                #trie
                _parse(s.as_bytes()).ok_or(())
            }
        }
    })
}
