use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream, Parser},
    parse_macro_input,
    spanned::Spanned,
    token::{Brace, Paren},
    DeriveInput, Error, Expr, FnArg, Ident, ItemFn, ItemStruct, Lit, Result, Token, Type,
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
            .map(|(ident, expr)| match expr {
                Expr::Lit(lit) => match &lit.lit {
                    Lit::Int(lit) if lit.suffix() == "pct" => {
                        let value = lit.base10_parse::<f32>().unwrap();
                        quote!(#ident: ::iocraft::Percent(#value).into())
                    }
                    Lit::Float(lit) if lit.suffix() == "pct" => {
                        let value = lit.base10_parse::<f32>().unwrap();
                        quote!(#ident: ::iocraft::Percent(#value).into())
                    }
                    _ => quote!(#ident: (#expr).into()),
                },
                _ => quote!(#ident: (#expr).into()),
            })
            .collect::<Vec<_>>();

        if !self.children.is_empty() {
            let children = self.children.iter().map(|child| quote!((#child).into()));
            props.push(quote!(children: vec![#(#children,)*]));
        }

        tokens.extend(quote! {
            {
                type Props = <#ty as ::iocraft::ElementType>::Props;
                ::iocraft::Element::<#ty>{
                    key: ::iocraft::ElementKey::new(),
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
pub fn element(input: TokenStream) -> TokenStream {
    let element = parse_macro_input!(input as ParsedElement);
    quote!(#element).into()
}

struct ParsedState {
    state: ItemStruct,
}

impl Parse for ParsedState {
    fn parse(input: ParseStream) -> Result<Self> {
        let state: ItemStruct = input.parse()?;
        Ok(Self { state })
    }
}

impl ToTokens for ParsedState {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let state = &self.state;
        let name = &state.ident;
        let field_assignments = state.fields.iter().map(|field| {
            let field_name = &field.ident;
            quote! { #field_name: owner.new_signal_with_default() }
        });

        tokens.extend(quote! {
            #state

            impl #name {
                fn new(owner: &mut ::iocraft::SignalOwner) -> Self {
                    Self {
                        #(#field_assignments,)*
                    }
                }
            }
        });
    }
}

#[proc_macro_attribute]
pub fn state(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let state = parse_macro_input!(item as ParsedState);
    quote!(#state).into()
}

struct ParsedHooks {
    hooks: ItemStruct,
}

impl Parse for ParsedHooks {
    fn parse(input: ParseStream) -> Result<Self> {
        let hooks: ItemStruct = input.parse()?;
        Ok(Self { hooks })
    }
}

impl ToTokens for ParsedHooks {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let hooks = &self.hooks;
        let name = &hooks.ident;

        let status_vars = hooks.fields.iter().map(|field| {
            let field_name = &field.ident;
            quote! { let #field_name = std::pin::Pin::new(&mut self.#field_name).poll_change(cx); }
        });
        let returns = hooks.fields.iter().map(|field| {
            let field_name = &field.ident;
            quote! {
                if #field_name.is_ready() {
                    return std::task::Poll::Ready(());
                }
            }
        });

        tokens.extend(quote! {
            #[derive(Default)]
            #hooks

            impl #name {
                fn poll_change(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
                    #(#status_vars)*
                    #(#returns)*
                    std::task::Poll::Pending
                }
            }
        });
    }
}

#[proc_macro_attribute]
pub fn hooks(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let hooks = parse_macro_input!(item as ParsedHooks);
    quote!(#hooks).into()
}

enum ComponentImplementationArg {
    State,
    Hooks,
}

struct ParsedComponent {
    f: ItemFn,
    state_type: Option<Box<Type>>,
    hooks_type: Option<Box<Type>>,
    args: Vec<ComponentImplementationArg>,
}

impl Parse for ParsedComponent {
    fn parse(input: ParseStream) -> Result<Self> {
        let f: ItemFn = input.parse()?;

        let mut state_type = None;
        let mut hooks_type = None;
        let mut args = Vec::new();

        for arg in &f.sig.inputs {
            match arg {
                FnArg::Typed(arg) => {
                    let name = arg.pat.to_token_stream().to_string();
                    match name.as_str() {
                        "state" => {
                            if state_type.is_some() {
                                return Err(Error::new(arg.span(), "duplicate `state` argument"));
                            }
                            match &*arg.ty {
                                Type::Reference(r) => {
                                    state_type = Some(r.elem.clone());
                                    args.push(ComponentImplementationArg::State);
                                }
                                _ => return Err(Error::new(arg.ty.span(), "invalid `state` type")),
                            }
                        }
                        "hooks" => {
                            if hooks_type.is_some() {
                                return Err(Error::new(arg.span(), "duplicate `hooks` argument"));
                            }
                            match &*arg.ty {
                                Type::Reference(r) => {
                                    hooks_type = Some(r.elem.clone());
                                    args.push(ComponentImplementationArg::Hooks);
                                }
                                _ => return Err(Error::new(arg.ty.span(), "invalid `hooks` type")),
                            }
                        }
                        _ => return Err(Error::new(arg.span(), "invalid argument")),
                    }
                }
                _ => return Err(Error::new(arg.span(), "invalid argument")),
            }
        }

        Ok(Self {
            f,
            state_type,
            hooks_type,
            args,
        })
    }
}

impl ToTokens for ParsedComponent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let vis = &self.f.vis;
        let name = &self.f.sig.ident;
        let args = &self.f.sig.inputs;
        let block = &self.f.block;
        let output = &self.f.sig.output;

        let state_decl = self.state_type.as_ref().map(|ty| quote!(state: #ty,));
        let state_init = self
            .state_type
            .as_ref()
            .map(|ty| quote!(state: #ty::new(&mut signal_owner),));

        let hooks_decl = self.hooks_type.as_ref().map(|ty| quote!(hooks: #ty,));
        let hooks_init = self
            .hooks_type
            .as_ref()
            .map(|ty| quote!(hooks: #ty::default(),));
        let hooks_status_check = self.hooks_type.as_ref().map(|_| {
            quote! {
                let hooks_status = std::pin::Pin::new(&mut self.hooks).poll_change(cx);
            }
        });
        let hooks_status_return = self.hooks_type.as_ref().map(|_| {
            quote! {
                if hooks_status.is_ready() {
                    return std::task::Poll::Ready(());
                }
            }
        });

        let impl_args = self
            .args
            .iter()
            .map(|arg| match arg {
                ComponentImplementationArg::State => quote!(&self.state),
                ComponentImplementationArg::Hooks => quote!(&mut self.hooks),
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            #vis struct #name {
                signal_owner: ::iocraft::SignalOwner,
                #state_decl
                #hooks_decl
            }

            impl #name {
                fn implementation(#args) #output #block
            }

            impl ::iocraft::Component for #name {
                type Props = ::iocraft::NoProps;

                fn new(_props: &Self::Props) -> Self {
                    let mut signal_owner = ::iocraft::SignalOwner::new();
                    Self {
                        #state_init
                        #hooks_init
                        signal_owner,
                    }
                }

                fn update(&mut self, _props: &Self::Props, updater: &mut ::iocraft::ComponentUpdater<'_>) {
                    let e = Self::implementation(#(#impl_args),*);
                    updater.update_children([e]);
                }

                fn poll_change(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
                    let signals_status = std::pin::Pin::new(&mut self.signal_owner).poll_change(cx);
                    #hooks_status_check

                    if signals_status.is_ready() {
                        return std::task::Poll::Ready(());
                    }
                    #hooks_status_return

                    std::task::Poll::Pending
                }
            }
        });
    }
}

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let component = parse_macro_input!(item as ParsedComponent);
    quote!(#component).into()
}

const LAYOUT_STYLE_FIELDS: &[(&str, &str)] = &[
    ("display", "::iocraft::Display"),
    ("width", "::iocraft::Size"),
    ("height", "::iocraft::Size"),
    ("min_width", "::iocraft::Size"),
    ("min_height", "::iocraft::Size"),
    ("max_width", "::iocraft::Size"),
    ("max_height", "::iocraft::Size"),
    ("padding", "::iocraft::Padding"),
    ("padding_top", "::iocraft::Padding"),
    ("padding_right", "::iocraft::Padding"),
    ("padding_bottom", "::iocraft::Padding"),
    ("padding_left", "::iocraft::Padding"),
    ("margin", "::iocraft::Margin"),
    ("margin_top", "::iocraft::Margin"),
    ("margin_right", "::iocraft::Margin"),
    ("margin_bottom", "::iocraft::Margin"),
    ("margin_left", "::iocraft::Margin"),
    ("overflow", "Option<::iocraft::Overflow>"),
    ("overflow_x", "Option<::iocraft::Overflow>"),
    ("overflow_y", "Option<::iocraft::Overflow>"),
    ("flex_direction", "::iocraft::FlexDirection"),
    ("flex_wrap", "::iocraft::FlexWrap"),
    ("flex_basis", "::iocraft::FlexBasis"),
    ("flex_grow", "f32"),
    ("flex_shrink", "Option<f32>"),
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
                    pub fn layout_style(&self) -> ::iocraft::LayoutStyle {
                        ::iocraft::LayoutStyle{
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
