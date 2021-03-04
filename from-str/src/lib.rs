//! Code generation for a compile-time trie-based mapping from strings to arbitrary values.

mod trie;

use std::collections::BTreeMap;
use std::io;

use quote::{quote, ToTokens};
use proc_macro2::{Literal, Ident, TokenStream, Span};

/// Generates a lookup function for all the key-value pairs contained in the tree.
///
/// # Examples
///
/// ```rust
/// # #![recursion_limit="128"]
/// # use quote::quote;
/// use enum_utils_from_str::StrMapFunc;
///
/// # fn main() {
/// // Compiling this trie into a lookup function...
/// let mut code = vec![];
/// StrMapFunc::new("custom_lookup", "bool")
///     .entries(vec![
///         ("yes", true),
///         ("yep", true),
///         ("no", false),
///     ])
///     .compile(&mut code);
///
/// // results in the following generated code.
///
/// # let generated = quote! {
/// fn custom_lookup(s: &[u8]) -> Option<bool> {
///     match s.len() {
///         2 => if s[0] == b'n' && s[1] == b'o' {
///             return Some(false);
///         },
///         3 => if s[0] == b'y' && s[1] == b'e' {
///             if s[2] == b'p' {
///                  return Some(true);
///             } else if s[2] == b's' {
///                 return Some(true);
///             }
///         },
///
///         _ => {}
///     }
///
///     None
/// }
/// # };
///
/// # assert_eq!(String::from_utf8(code).unwrap(), format!("{}", generated));
/// # }
/// ```
#[derive(Clone)]
pub struct StrMapFunc {
    atoms: Forest<TokenStream>,
    func_name: Ident,
    ret_ty: TokenStream,
    case: Case,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Sensitive,
    Insensitive,
}

impl StrMapFunc {
    pub fn new(func_name: &str, ret_ty: &str) -> Self {
        StrMapFunc {
            atoms: Default::default(),
            func_name: Ident::new(func_name, Span::call_site()),
            ret_ty: ret_ty.parse().unwrap(),
            case: Case::Sensitive,
        }
    }

    pub fn case(&mut self, case: Case) -> &mut Self {
        self.case = case;
        self
    }

    pub fn entry(&mut self, k: &str, v: impl ToTokens) -> &mut Self {
        self.atoms.insert(k.as_bytes(), v.into_token_stream());
        self
    }

    pub fn entries<'a, V: 'a>(&mut self, entries: impl IntoIterator<Item = (&'a str, V)>) -> &mut Self
        where V: ToTokens,
    {
        for (s, v) in entries.into_iter() {
            self.entry(s, v);
        }

        self
    }

    pub fn compile(&self, mut w: impl io::Write) -> io::Result<()> {
        let tokens = self.into_token_stream();
        w.write_all(format!("{}", tokens).as_bytes())
    }
}

impl ToTokens for StrMapFunc {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let StrMapFunc { func_name, ret_ty, atoms, case } = self;

        let match_arms = atoms.0.iter()
            .map(|(&len, trie)| {
                let branch = Forest::branch_tokens(trie, *case == Case::Insensitive);
                let len = Literal::usize_unsuffixed(len);

                quote!(#len => #branch)
            });

        let body = quote! {
            match s.len() {
                #( #match_arms, )*
                _ => {}
            }

            ::core::option::Option::None
        };

        tokens.extend(quote! {
            fn #func_name(s: &[u8]) -> ::core::option::Option<#ret_ty> {
                #body
            }
        });
    }
}

/// A set of tries where each trie only stores strings of a single length.
#[derive(Debug, Clone)]
pub struct Forest<T>(BTreeMap<usize, trie::Node<T>>);

impl<T> Default for Forest<T> {
    fn default() -> Self {
        Forest(Default::default())
    }
}

impl<T> Forest<T> {
    pub fn get(&mut self, bytes: &[u8]) -> Option<&T> {
        self.0.get(&bytes.len())
            .and_then(|n| n.get(bytes))
    }

    pub fn insert(&mut self, bytes: &[u8], value: T) -> Option<T> {
        let node = self.0.entry(bytes.len()).or_default();
        node.insert(bytes, value)
    }
}

fn byte_literal(b: u8) -> TokenStream {
    if b < 128 {
        let c: String = char::from(b).escape_default().collect();
        format!("b'{}'", c).parse().unwrap()
    } else {
        Literal::u8_unsuffixed(b).into_token_stream()
    }
}

impl<T> Forest<T>
    where T: ToTokens
{
    fn branch_tokens(node: &trie::Node<T>, ignore_case: bool) -> TokenStream {
        use trie::TraversalOrder::*;

        let mut tok = vec![TokenStream::new()];
        let mut depth = 0;
        let mut is_first_child = true;
        let mut dfs = node.dfs();
        while let Some((order, node)) = dfs.next() {
            if node.bytes.is_empty() {
                continue;
            }

            match order {
                Pre => {
                    if !is_first_child {
                        tok.last_mut().unwrap().extend(quote!(else));
                        is_first_child = true;
                    }

                    let i = (depth..depth+node.bytes.len()).map(Literal::usize_unsuffixed);
                    let b = node.bytes.iter().cloned().map(byte_literal);

                    if !ignore_case {
                        tok.last_mut().unwrap().extend(quote!(if #( s[#i] == #b )&&*));
                    } else {
                        tok.last_mut().unwrap().extend(quote!(if #( s[#i].eq_ignore_ascii_case(&#b) )&&*));
                    }

                    tok.push(TokenStream::new());
                    depth += node.bytes.len();

                    if let Some(v) = node.value {
                        // TODO: debug_assert_eq!(dfs.next().0, Post);

                        tok.last_mut().unwrap().extend(quote!(return ::core::option::Option::Some(#v);));
                    }
                }

                Post => {
                    let body = tok.pop().unwrap();
                    tok.last_mut().unwrap().extend(quote!({ #body }));
                    depth -= node.bytes.len();
                    is_first_child = false;
                }
            }
        }

        let ret = tok.pop().unwrap();
        assert!(tok.is_empty());
        ret
    }
}
