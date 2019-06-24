use failure::format_err;
use proc_macro2::{TokenStream, Span};
use quote::quote;

use crate::attr::{Enum, ErrorList};

pub fn derive_try_from_repr(input: &syn::DeriveInput) -> Result<TokenStream, ErrorList> {
    let Enum { name, variants, primitive_repr, .. } = Enum::parse(input)?;

    let mut errors = ErrorList::new();
    let repr = match primitive_repr {
        Ok(Some((_, repr))) => repr,

        Ok(None) => bail_list!("`#[repr(...)]` must be specified to derive `TryFrom`"),
        Err(e) => {
            errors.push_back(e);
            return Err(errors);
        }
    };

    for (v, _) in variants.iter() {
        if v.fields != syn::Fields::Unit {
            errors.push_back(format_err!("Variant cannot have fields"));
            continue;
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let consts = variants.iter()
        .map(|(v, _)| {
            let s = "DISCRIMINANT_".to_owned() + &v.ident.to_string();
            syn::Ident::new(s.as_str(), Span::call_site())
        });

    let ctors = variants.iter()
        .map(|(v, _)| {
            let v = &v.ident;
            quote!(#name::#v)
        });

    // `as` casts are not valid as part of a pattern, so we need to do define new `consts` to hold
    // them.
    let const_defs = consts.clone()
        .zip(ctors.clone())
        .map(|(v, ctor)|  quote!(const #v: #repr = #ctor as #repr));

    Ok(quote! {
        impl ::std::convert::TryFrom<#repr> for #name {
            type Error = ();

            #[allow(non_upper_case_globals)]
            fn try_from(d: #repr) -> Result<Self, Self::Error> {

                #( #const_defs; )*

                match d {
                    #( #consts => Ok(#ctors), )*
                    _ => Err(())
                }
            }
        }
    })
}

pub fn derive_repr_from(input: &syn::DeriveInput) -> Result<TokenStream, ErrorList> {
    let Enum { name, variants, primitive_repr, .. } = Enum::parse(input)?;

    let mut errors = ErrorList::new();
    let repr = match primitive_repr {
        Ok(Some((_, repr))) => repr,

        Ok(None) => bail_list!("`#[repr(...)]` must be specified to derive `TryFrom`"),
        Err(e) => {
            errors.push_back(e);
            return Err(errors);
        }
    };

    for (v, _) in variants.iter() {
        if v.fields != syn::Fields::Unit {
            errors.push_back(format_err!("Variant cannot have fields"));
            continue;
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(quote! {
        impl ::std::convert::From<#name> for #repr {
            fn from(d: #name) -> Self {
                d as #repr
            }
        }
    })
}
