use std::ops::{Range, RangeInclusive};

use failure::format_err;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::attr::{Discriminant, Enum, ErrorList};

enum IterImpl {
    Empty,
    Range {
        repr: syn::Path,
        range: Range<Discriminant>,
    },
    RangeInclusive {
        repr: syn::Path,
        range: RangeInclusive<Discriminant>,
    },
    Slice(Vec<TokenStream>),
}

impl IterImpl {
    /// Constructs the fastest `IterImpl` for the given set of discriminants.
    ///
    /// If the discriminants form a single, contiguous, increasing run, we will create a
    /// `Range` (or `RangeInclusive`) containing the discriminants as the `#[repr(...)]` of the
    /// enum.
    fn for_enum(Enum { name, variants, discriminants, primitive_repr, .. }: &Enum) -> Result<Self, ErrorList> {
        // See if we can generate a fast, transmute-based iterator.
        if let Some(discriminants) = discriminants {
            let is_zst = discriminants.len() <= 1;

            if let Ok(Some((repr, repr_path))) = primitive_repr {
                let unskipped_discriminants: Vec<_> = discriminants
                    .iter()
                    .cloned()
                    .zip(variants.iter())
                    .filter(|(_, (_, attr))| !attr.skip)
                    .map(|(d, _)| d)
                    .collect();

                if unskipped_discriminants.is_empty() {
                    return Ok(IterImpl::Empty);
                }

                if !is_zst {
                    if let Some(range) = detect_contiguous_run(unskipped_discriminants.into_iter()) {
                        // If range.end() is less than the maximum value of the primitive repr, we can
                        // use the (faster) non-inclusive `Range`
                        let end = *range.end();
                        if end < 0 || repr.max_value().map_or(false, |max| (end as u128) < max) {
                            return Ok(IterImpl::Range {
                                repr: repr_path.clone(),
                                range: *range.start()..(end + 1),
                            })
                        }

                        return Ok(IterImpl::RangeInclusive {
                            repr: repr_path.clone(),
                            range,
                        })
                    }
                }
            }
        }

        // ...if not, fall back to the slice based one.
        let mut errors = ErrorList::new();
        let unskipped_variants: Vec<_> = variants
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

        if !errors.is_empty() {
            return Err(errors);
        }

        if unskipped_variants.is_empty() {
            return Ok(IterImpl::Empty);
        }

        Ok(IterImpl::Slice(unskipped_variants))
    }

    fn tokens(&self, ty: &syn::Ident) -> TokenStream {
        let body = match self {
            IterImpl::Empty => quote! {
                ::std::iter::empty()
            },

            IterImpl::Range { range, repr } => {
                let start = Literal::i128_unsuffixed(range.start);
                let end = Literal::i128_unsuffixed(range.end);

                quote! {
                    let start: #repr = #start;
                    let end: #repr = #end;
                    (start .. end).map(|discrim| unsafe { ::std::mem::transmute(discrim) })
                }
            },

            IterImpl::RangeInclusive { range, repr } => {
                let start = Literal::i128_unsuffixed(*range.start());
                let end = Literal::i128_unsuffixed(*range.end());
                quote! {
                    let start: #repr = #start;
                    let end: #repr = #end;
                    (start ..= end).map(|discrim| unsafe { ::std::mem::transmute(discrim) })
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
/// run. Returns `None` if there were no discriminants or they were not contiguous.
fn detect_contiguous_run(mut discriminants: impl Iterator<Item = Discriminant>)
    -> Option<RangeInclusive<Discriminant>>
{
    let first = discriminants.next()?;

    let mut last = first;
    while let Some(next) = discriminants.next() {
        if last.checked_add(1)? != next {
            return None;
        }

        last = next
    }

    Some(first..=last)
}

pub fn derive(input: &syn::DeriveInput) -> Result<TokenStream, ErrorList> {
    let input = Enum::parse(input)?;
    let imp = IterImpl::for_enum(&input)?;
    Ok(imp.tokens(&input.name))
}
