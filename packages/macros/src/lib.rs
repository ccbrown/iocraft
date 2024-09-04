use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream, Parser},
    parse_macro_input,
    token::{Brace, Paren},
    DeriveInput, Expr, Ident, Result, Token, Type,
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
            let mut is_first = true;
            while !props_input.is_empty() {
                if !is_first {
                    props_input.parse::<Token![,]>()?;
                }
                let ident: Ident = props_input.parse()?;
                props_input.parse::<Token![:]>()?;
                let expr: Expr = props_input.parse()?;
                props.push((ident, expr));
                is_first = false;
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
                    key: ::flashy_io::ElementKey::new(),
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

const LAYOUT_STYLE_FIELDS: &[(&str, &str)] = &[
    ("display", "::flashy_io::Display"),
    ("padding", "Option<u32>"),
    ("padding_top", "Option<u32>"),
    ("padding_right", "Option<u32>"),
    ("padding_bottom", "Option<u32>"),
    ("padding_left", "Option<u32>"),
    ("margin", "Option<u32>"),
    ("margin_top", "Option<u32>"),
    ("margin_right", "Option<u32>"),
    ("margin_bottom", "Option<u32>"),
    ("margin_left", "Option<u32>"),
    ("overflow", "Option<::flashy_io::Overflow>"),
    ("overflow_x", "Option<::flashy_io::Overflow>"),
    ("overflow_y", "Option<::flashy_io::Overflow>"),
    ("flex_direction", "::flashy_io::FlexDirection"),
];

#[proc_macro_attribute]
pub fn with_layout_style_props(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    for (field_name, field_type) in LAYOUT_STYLE_FIELDS {
                        let field_name = Ident::new(field_name, Span::call_site());
                        let field_type = syn::parse_str::<Type>(field_type).unwrap();
                        fields.named.push(
                            syn::Field::parse_named
                                .parse2(quote! { pub #field_name: #field_type })
                                .unwrap(),
                        );
                    }
                }
                _ => (),
            }

            let struct_name = &ast.ident;
            let field_assignments = LAYOUT_STYLE_FIELDS.iter().map(|(field_name, _)| {
                let field_name = Ident::new(field_name, Span::call_site());
                quote! { #field_name: self.#field_name }
            });

            return quote! {
                #ast

                impl #struct_name {
                    pub fn layout_style(&self) -> ::flashy_io::LayoutStyle {
                        ::flashy_io::LayoutStyle{
                            #(#field_assignments,)*
                        }
                    }
                }
            }
            .into();
        }
        _ => panic!("`add_field` has to be used with structs "),
    }
}
