use std::ops;

use failure::format_err;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::attr::{Discriminant, Enum, ErrorList, reprs};

enum IterImpl {
    Range {
        repr: TokenStream,
        range: ops::Range<Discriminant>,
    },
    Slice(Vec<TokenStream>),
}

impl IterImpl {
    fn tokens(&self, ty: &syn::Ident) -> TokenStream {
        let body = match self {
            IterImpl::Range { range, repr } => {
                let start = Literal::i128_unsuffixed(range.start);
                let end = Literal::i128_unsuffixed(range.end);
                quote! {
                    let start: #repr = #start;
                    let end: #repr = #end;
                    (start .. end).map(|discrim| unsafe { ::std::mem::transmute(discrim) })
                }
            },

            IterImpl::Slice(variants) => quote! {
                const VARIANTS: &[#ty] = &[#( #variants ),*];

                VARIANTS.iter().cloned()
            },
        };

        quote! {
            impl #ty {
                fn iter() -> impl Iterator<Item = #ty> + Clone {
                    #body
                }
            }
        }
    }
}

/// Returns a range containing the discriminants of this enum if they comprise a single, contiguous
/// block. Returns `None` if there were no discriminants or they were not contigous.
fn contiguous_range(mut it: impl Iterator<Item = Discriminant>) -> Option<ops::Range<Discriminant>> {
    let start = it.next()?;
    let mut end = start;
    while let Some(next) = it.next() {
        if next != end.checked_add(1).expect("Discriminant overflowed") {
            return None;
        }

        end = next
    }

    // TODO: use range inclusive to handle massive enums.
    end = end.checked_add(1).expect("Last discriminant is equal to i128::MAX");
    Some(start..end)
}

pub fn derive(input: &syn::DeriveInput) -> Result<TokenStream, ErrorList> {
    let Enum { name, variants, discriminants, .. } = Enum::parse(input)?;
    let reprs = reprs(input.attrs.iter());

    let mut errors = ErrorList::default();
    let variants: Vec<_> = variants
        .iter()
        .filter_map(|(v, attr)| {
            if attr.skip {
                return None;
            }

            if v.fields != syn::Fields::Unit {
                errors.push_back(format_err!("An (unskipped) variant cannot have fields"));
                return None;
            }

            let vident = &v.ident;
            Some(quote!(#name::#vident))
        })
        .collect();

    if variants.is_empty() {
        errors.push_back(format_err!("Cannot derive iter on enum with no fields"));
    }

    let contiguous_range = discriminants.and_then(|d| contiguous_range(d.into_iter()));
    let imp = match contiguous_range {
        // Enums with a single variant have size 0.
        Some(_) if variants.len() == 1 => IterImpl::Slice(variants),

        // TODO: Implement `IterImpl::Range` and use it here instead?
        Some(range) => match reprs.as_slice() {
            [repr] => IterImpl::Range { range, repr: repr.clone() },
            _ => IterImpl::Slice(variants),
        }

        None => IterImpl::Slice(variants),
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(imp.tokens(&name))
}
