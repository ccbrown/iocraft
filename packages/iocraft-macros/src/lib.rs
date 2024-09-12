use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream, Parser},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Comma, Paren},
    DeriveInput, Error, Expr, FieldValue, FnArg, GenericParam, Ident, ItemFn, ItemStruct, Lifetime,
    Lit, Result, Token, Type, TypePath,
};

enum ParsedElementChild {
    Element(ParsedElement),
    Expr(Expr),
}

struct ParsedElement {
    ty: TypePath,
    props: Punctuated<FieldValue, Comma>,
    children: Vec<ParsedElementChild>,
}

impl Parse for ParsedElement {
    /// Parses a single element of the form:
    ///
    /// MyComponent(my_prop: "foo") {
    ///     // children
    /// }
    fn parse(input: ParseStream) -> Result<Self> {
        let ty: TypePath = input.parse()?;

        let props = if input.peek(Paren) {
            let props_input;
            parenthesized!(props_input in input);
            Punctuated::parse_terminated(&props_input)?
        } else {
            Punctuated::new()
        };

        let mut children = Vec::new();
        if input.peek(Brace) {
            let children_input;
            braced!(children_input in input);
            while !children_input.is_empty() {
                if children_input.peek(Token![#]) {
                    children_input.parse::<Token![#]>()?;
                    let child_input;
                    parenthesized!(child_input in children_input);
                    children.push(ParsedElementChild::Expr(child_input.parse()?));
                } else {
                    children.push(ParsedElementChild::Element(children_input.parse()?));
                }
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

        let props = self
            .props
            .iter()
            .map(|FieldValue { member, expr, .. }| match expr {
                Expr::Lit(lit) => match &lit.lit {
                    Lit::Int(lit) if lit.suffix() == "pct" => {
                        let value = lit.base10_parse::<f32>().unwrap();
                        quote!(#member: ::iocraft::Percent(#value).into())
                    }
                    Lit::Float(lit) if lit.suffix() == "pct" => {
                        let value = lit.base10_parse::<f32>().unwrap();
                        quote!(#member: ::iocraft::Percent(#value).into())
                    }
                    _ => quote!(#member: (#expr).into()),
                },
                _ => quote!(#member: (#expr).into()),
            })
            .collect::<Vec<_>>();

        let set_children = if !self.children.is_empty() {
            let children = self.children.iter().map(|child| match child {
                ParsedElementChild::Element(child) => quote!(#child),
                ParsedElementChild::Expr(expr) => quote!(#expr),
            });
            Some(quote! {
                #(::iocraft::extend_with_elements(&mut _iocraft_element.props.children, #children);)*
            })
        } else {
            None
        };

        tokens.extend(quote! {
            {
                type Props<'a> = <#ty as ::iocraft::ElementType>::Props<'a>;
                let mut _iocraft_element = ::iocraft::Element::<#ty>{
                    key: ::iocraft::ElementKey::new(),
                    props: Props{
                        #(#props,)*
                        ..core::default::Default::default()
                    },
                };
                #set_children
                _iocraft_element
            }
        });
    }
}

#[proc_macro]
pub fn element(input: TokenStream) -> TokenStream {
    let element = parse_macro_input!(input as ParsedElement);
    quote!(#element).into()
}

struct ParsedCovariant {
    def: ItemStruct,
}

impl Parse for ParsedCovariant {
    fn parse(input: ParseStream) -> Result<Self> {
        let def: ItemStruct = input.parse()?;
        Ok(Self { def })
    }
}

impl ToTokens for ParsedCovariant {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let def = &self.def;
        let name = &def.ident;
        let where_clause = &def.generics.where_clause;

        let has_generics = def.generics.params.len() > 0;
        let lifetime_generic_count = def
            .generics
            .params
            .iter()
            .filter(|param| match param {
                GenericParam::Lifetime(_) => true,
                _ => false,
            })
            .count();

        let generics = &def.generics;

        let generics_names = def.generics.params.iter().map(|param| match param {
            GenericParam::Type(ty) => {
                let name = &ty.ident;
                quote!(#name)
            }
            GenericParam::Lifetime(lt) => {
                let name = &lt.lifetime;
                quote!(#name)
            }
            GenericParam::Const(c) => {
                let name = &c.ident;
                quote!(#name)
            }
        });
        let bracketed_generic_names = match has_generics {
            true => quote!(<#(#generics_names),*>),
            false => quote!(),
        };

        // If the struct is generic over lifetimes, emit code that will break things at compile
        // time when the struct is not covariant with respect to its lifetimes.
        if lifetime_generic_count > 0 {
            let generic_decls = {
                let mut lifetime_index = 0;
                def.generics.params.iter().map(move |param| match param {
                    GenericParam::Lifetime(_) => {
                        let a = Lifetime::new(
                            format!("'a{}", lifetime_index).as_str(),
                            Span::call_site(),
                        );
                        let b = Lifetime::new(
                            format!("'b{}", lifetime_index).as_str(),
                            Span::call_site(),
                        );
                        lifetime_index += 1;
                        quote!(#a, #b: #a)
                    }
                    _ => quote!(#param),
                })
            };

            let test_args = ["a", "b"].iter().map(|arg| {
                let mut lifetime_index = 0;
                let generic_params = def.generics.params.iter().map(|param| match param {
                    GenericParam::Type(ty) => {
                        let name = &ty.ident;
                        quote!(#name)
                    }
                    GenericParam::Lifetime(_) => {
                        let lt = Lifetime::new(
                            format!("'{}{}", arg, lifetime_index).as_str(),
                            Span::call_site(),
                        );
                        lifetime_index += 1;
                        quote!(#lt)
                    }
                    GenericParam::Const(c) => {
                        let name = &c.ident;
                        quote!(#name)
                    }
                });
                let arg_ident = Ident::new(arg, Span::call_site());
                quote!(#arg_ident: &#name<#(#generic_params),*>)
            });

            tokens.extend(quote! {
                const _: () = {
                    fn take_two<T>(_a: T, _b: T) {}

                    fn test_type_covariance<#(#generic_decls),*>(#(#test_args),*) {
                        take_two(a, b)
                    }
                };
            });
        }

        tokens.extend(quote! {
            unsafe impl #generics ::iocraft::Covariant for #name #bracketed_generic_names #where_clause {}
        });
    }
}

#[proc_macro_derive(Covariant)]
pub fn derive_covariant_type(item: TokenStream) -> TokenStream {
    let props = parse_macro_input!(item as ParsedCovariant);
    quote!(#props).into()
}

struct ParsedProps {
    props: ItemStruct,
}

impl Parse for ParsedProps {
    fn parse(input: ParseStream) -> Result<Self> {
        let props: ItemStruct = input.parse()?;
        Ok(Self { props })
    }
}

impl ToTokens for ParsedProps {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let props = &self.props;

        tokens.extend(quote! {
            #[derive(Default, ::iocraft::Covariant)]
            #props
        });
    }
}

#[proc_macro_attribute]
pub fn props(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let props = parse_macro_input!(item as ParsedProps);
    quote!(#props).into()
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
        let pre_component_updates = hooks.fields.iter().map(|field| {
            let field_name = &field.ident;
            quote! {
                self.#field_name.pre_component_update(updater);
            }
        });
        let post_component_updates = hooks.fields.iter().map(|field| {
            let field_name = &field.ident;
            quote! {
                self.#field_name.post_component_update(updater);
            }
        });

        tokens.extend(quote! {
            #[derive(Default)]
            #hooks

            impl #name {
                fn poll_change(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context) -> std::task::Poll<()> {
                    #(#status_vars)*
                    #(#returns)*
                    std::task::Poll::Pending
                }

                fn pre_component_update(&mut self, updater: &mut ::iocraft::ComponentUpdater) {
                    #(#pre_component_updates)*
                }

                fn post_component_update(&mut self, updater: &mut ::iocraft::ComponentUpdater) {
                    #(#post_component_updates)*
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

struct ParsedContext {
    context: ItemStruct,
}

impl Parse for ParsedContext {
    fn parse(input: ParseStream) -> Result<Self> {
        let context: ItemStruct = input.parse()?;
        Ok(Self { context })
    }
}

fn is_option(ty: Type) -> bool {
    match ty {
        Type::Path(path) => path.path.segments.last().map_or(false, |segment| {
            segment.ident == "Option" && !segment.arguments.is_empty()
        }),
        _ => false,
    }
}

impl ToTokens for ParsedContext {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let context = &self.context;
        let name = &context.ident;

        let field_assignments = context.fields.iter().map(|field| {
            let field_name = &field.ident;
            if is_option(field.ty.clone()) {
                quote! { #field_name: updater.get_context() }
            } else {
                let err_msg = format!(
                    "missing required context for {}",
                    field_name.as_ref().unwrap()
                );
                quote! { #field_name: updater.get_context().expect(#err_msg) }
            }
        });

        tokens.extend(quote! {
            #context

            impl<'a> #name<'a> {
                fn new(updater: &'a ::iocraft::ComponentUpdater) -> Self {
                    Self {
                        #(#field_assignments,)*
                    }
                }
            }
        });
    }
}

#[proc_macro_attribute]
pub fn context(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let context = parse_macro_input!(item as ParsedContext);
    quote!(#context).into()
}

enum ComponentImplementationArg {
    State,
    Hooks,
    Props,
    Context,
}

struct ParsedComponent {
    f: ItemFn,
    props_type: Option<Box<Type>>,
    state_type: Option<Box<Type>>,
    hooks_type: Option<Box<Type>>,
    context_type: Option<Box<Type>>,
    args: Vec<ComponentImplementationArg>,
}

impl Parse for ParsedComponent {
    fn parse(input: ParseStream) -> Result<Self> {
        let f: ItemFn = input.parse()?;

        let mut props_type = None;
        let mut state_type = None;
        let mut hooks_type = None;
        let mut context_type = None;
        let mut args = Vec::new();

        for arg in &f.sig.inputs {
            match arg {
                FnArg::Typed(arg) => {
                    let name = arg.pat.to_token_stream().to_string();
                    match name.as_str() {
                        "props" => {
                            if props_type.is_some() {
                                return Err(Error::new(arg.span(), "duplicate `props` argument"));
                            }
                            match &*arg.ty {
                                Type::Reference(r) => {
                                    props_type = Some(r.elem.clone());
                                    args.push(ComponentImplementationArg::Props);
                                }
                                _ => return Err(Error::new(arg.ty.span(), "invalid `props` type")),
                            }
                        }
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
                        "context" => {
                            if context_type.is_some() {
                                return Err(Error::new(arg.span(), "duplicate `context` argument"));
                            }
                            match &*arg.ty {
                                Type::Path(_) => {
                                    context_type = Some(arg.ty.clone());
                                    args.push(ComponentImplementationArg::Context);
                                }
                                _ => {
                                    return Err(Error::new(arg.ty.span(), "invalid `context` type"))
                                }
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
            props_type,
            state_type,
            hooks_type,
            context_type,
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
        let generics = &self.f.sig.generics;

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
        let hooks_pre_component_update = self
            .hooks_type
            .as_ref()
            .map(|_| quote! { self.hooks.pre_component_update(updater); });
        let hooks_post_component_update = self
            .hooks_type
            .as_ref()
            .map(|_| quote! { self.hooks.post_component_update(updater); });

        let props_type_name = self
            .props_type
            .as_ref()
            .map(|ty| quote!(#ty))
            .unwrap_or_else(|| quote!(::iocraft::NoProps));

        let impl_args = self
            .args
            .iter()
            .map(|arg| match arg {
                ComponentImplementationArg::State => quote!(&self.state),
                ComponentImplementationArg::Hooks => quote!(&mut self.hooks),
                ComponentImplementationArg::Props => quote!(props),
                ComponentImplementationArg::Context => {
                    let type_name = self.context_type.as_ref().unwrap();
                    quote!(#type_name::new(updater))
                }
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            #vis struct #name {
                signal_owner: ::iocraft::SignalOwner,
                #state_decl
                #hooks_decl
            }

            impl #name {
                fn implementation #generics (#args) #output #block
            }

            impl ::iocraft::Component for #name {
                type Props<'a> = #props_type_name;

                fn new(_props: &Self::Props<'_>) -> Self {
                    let mut signal_owner = ::iocraft::SignalOwner::new();
                    Self {
                        #state_init
                        #hooks_init
                        signal_owner,
                    }
                }

                fn update(&mut self, props: &Self::Props<'_>, updater: &mut ::iocraft::ComponentUpdater) {
                    #hooks_pre_component_update
                    {
                        let e = Self::implementation(#(#impl_args),*).into();
                        updater.update_children([&e], None);
                    }
                    #hooks_post_component_update
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
    ("flex_direction", "::iocraft::FlexDirection"),
    ("flex_wrap", "::iocraft::FlexWrap"),
    ("flex_basis", "::iocraft::FlexBasis"),
    ("flex_grow", "f32"),
    ("flex_shrink", "Option<f32>"),
    ("align_items", "Option<::iocraft::AlignItems>"),
    ("align_content", "Option<::iocraft::AlignContent>"),
    ("justify_content", "Option<::iocraft::JustifyContent>"),
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

            let where_clause = &ast.generics.where_clause;

            let has_generics = ast.generics.params.len() > 0;
            let generics = &ast.generics;

            let generics_names = ast.generics.params.iter().map(|param| match param {
                GenericParam::Type(ty) => {
                    let name = &ty.ident;
                    quote!(#name)
                }
                GenericParam::Lifetime(lt) => {
                    let name = &lt.lifetime;
                    quote!(#name)
                }
                GenericParam::Const(c) => {
                    let name = &c.ident;
                    quote!(#name)
                }
            });
            let bracketed_generic_names = match has_generics {
                true => quote!(<#(#generics_names),*>),
                false => quote!(),
            };

            return quote! {
                #ast

                impl #generics #struct_name #bracketed_generic_names #where_clause {
                    pub fn layout_style(&self) -> ::iocraft::LayoutStyle {
                        ::iocraft::LayoutStyle{
                            #(#field_assignments,)*
                        }
                    }
                }
            }
            .into();
        }
        _ => panic!("`with_layout_style_props` can only be used with structs "),
    }
}
