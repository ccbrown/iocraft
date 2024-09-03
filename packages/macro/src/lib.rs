use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::{Brace, Paren},
    Expr, Ident, Result, Token, Type,
};

struct ParsedElement {
    ty: Type,
    props: Vec<(Ident, Expr)>,
    children: Vec<ParsedElement>,
}

impl Parse for ParsedElement {
    /// Parses a single element of the form:
    ///
    /// MyComponent(my_prop: "foo") {
    ///     // children
    /// }
    fn parse(input: ParseStream) -> Result<Self> {
        let ty: Type = input.parse()?;

        let mut props = Vec::new();
        if input.peek(Paren) {
            let props_input;
            parenthesized!(props_input in input);
            while !props_input.is_empty() {
                let ident: Ident = props_input.parse()?;
                props_input.parse::<Token![:]>()?;
                let expr: Expr = props_input.parse()?;
                props.push((ident, expr));
            }
        }

        let mut children = Vec::new();
        if input.peek(Brace) {
            let children_input;
            braced!(children_input in input);
            while !children_input.is_empty() {
                children.push(children_input.parse()?);
            }
        }

        Ok(Self {
            props,
            ty,
            children,
        })
    }
}

impl ToTokens for ParsedElement {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty = &self.ty;

        let mut props = self
            .props
            .iter()
            .map(|(ident, expr)| quote!(#ident: (#expr).into()))
            .collect::<Vec<_>>();

        if !self.children.is_empty() {
            let children = self.children.iter().map(|child| quote!((#child).into()));
            props.push(quote!(children: vec![#(#children,)*]));
        }

        tokens.extend(quote! {
            {
                type Props = <#ty as ::flashy_io::ElementType>::Props;
                ::flashy_io::Element::<#ty>{
                    key: "foo".to_string(),
                    props: Props{
                        #(#props,)*
                        ..core::default::Default::default()
                    },
                }
            }
        });
    }
}

#[proc_macro]
pub fn flashy(input: TokenStream) -> TokenStream {
    let element = parse_macro_input!(input as ParsedElement);
    quote!(#element).into()
}
