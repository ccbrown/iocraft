//! This crate defines the macros used by `iocraft`.

#![warn(missing_docs)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, ToTokens};
use std::sync::atomic::{AtomicU64, Ordering};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream, Parser},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Comma, Paren},
    DeriveInput, Error, Expr, FieldValue, Fields, FnArg, GenericParam, Generics, Ident, ItemFn,
    ItemStruct, Lifetime, Lit, Member, Pat, Result, Token, Type, TypePath, WhereClause,
    WherePredicate,
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

        // Use module_path!() combined with a deterministic counter to ensure
        // keys are unique both within a compilation unit and across crates.
        // module_path!() resolves to the caller's module path at compile time,
        // providing cross-crate uniqueness. The counter provides within-crate
        // uniqueness. This also ensures reproducible builds (no random UUIDs).
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

        let key = self
            .props
            .iter()
            .find_map(|FieldValue { member, expr, .. }| match member {
                Member::Named(ident) if ident == "key" => {
                    Some(quote!((module_path!(), #counter, #expr)))
                }
                _ => None,
            })
            .unwrap_or_else(|| quote!((module_path!(), #counter)));

        let prop_setters = self
            .props
            .iter()
            .filter_map(|FieldValue { member, expr, .. }| match member {
                Member::Named(ident) if ident == "key" => None,
                _ => Some(match expr {
                    Expr::Lit(lit) => match &lit.lit {
                        Lit::Int(lit) if lit.suffix() == "pct" => {
                            let value = lit.base10_parse::<f32>().unwrap();
                            quote!(.#member(::iocraft::Percent(#value)))
                        }
                        Lit::Float(lit) if lit.suffix() == "pct" => {
                            let value = lit.base10_parse::<f32>().unwrap();
                            quote!(.#member(::iocraft::Percent(#value)))
                        }
                        _ => quote!(.#member(#expr)),
                    },
                    _ => quote!(.#member(#expr)),
                }),
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
                let mut _iocraft_props: Props = Props::__iocraft_builder()
                    #(#prop_setters)*
                    .__iocraft_build();
                let mut _iocraft_element = ::iocraft::Element::<#ty>{
                    key: ::iocraft::ElementKey::new(#key),
                    props: _iocraft_props,
                };
                #set_children
                _iocraft_element
            }
        });
    }
}

/// Used to declare an element and its properties.
///
/// Elements are declared starting with their type. Properties are optional unless their
/// [`Props`] derive marks them with `#[iocraft(required)]`, so the simplest use is just a type name:
///
/// ```
/// # use iocraft::prelude::*;
/// # fn my_element() -> Element<'static, View> {
/// element!(View)
/// # }
/// ```
///
/// This will evaluate to an `Element<'static, View>` with no properties set.
///
/// To specify properties, you can add them in a parenthesized block after the type name:
///
/// ```
/// # use iocraft::prelude::*;
/// # fn my_element() -> Element<'static, View> {
/// element! {
///     View(width: 80, height: 24, background_color: Color::Green)
/// }
/// # }
/// ```
///
/// If the element has a `children` property, you can pass one or more child elements in braces like so:
///
/// ```
/// # use iocraft::prelude::*;
/// # fn my_element() -> Element<'static, View> {
/// element! {
///     View {
///         Text(content: "Hello, world!")
///     }
/// }
/// # }
/// ```
///
/// Lastly, you can use Rust to conditionally add child elements via `#()` blocks that evaluate
/// to any iterator type:
///
/// ```
/// # use iocraft::prelude::*;
/// # fn my_element(show_greeting: bool) -> Element<'static, View> {
/// element! {
///     View {
///         #(if show_greeting {
///             Some(element! {
///                 Text(content: "Hello, world!")
///             })
///         } else {
///             None
///         })
///     }
/// }
/// # }
/// ```
///
/// If you're rendering a dynamic UI, you will want to ensure that when adding multiple
/// elements via an iterator a unique key is specified for each one. Otherwise, the elements
/// may not correctly maintain their state across renders. This is done using the special `key`
/// property, which can be given to any element:
///
/// ```
/// # use iocraft::prelude::*;
/// # struct User { id: i32, name: String }
/// # fn my_element(users: Vec<User>) -> Element<'static, View> {
/// element! {
///     View {
///         #(users.iter().map(|user| element! {
///             View(key: user.id, flex_direction: FlexDirection::Column) {
///                 Text(content: format!("Hello, {}!", user.name))
///             }
///         }))
///     }
/// }
/// # }
/// ```
#[proc_macro]
pub fn element(input: TokenStream) -> TokenStream {
    let element = parse_macro_input!(input as ParsedElement);
    quote!(#element).into()
}

struct ParsedProps {
    def: ItemStruct,
    required_fields: Vec<bool>,
}

impl Parse for ParsedProps {
    fn parse(input: ParseStream) -> Result<Self> {
        let def: ItemStruct = input.parse()?;
        if matches!(def.fields, Fields::Unnamed(_)) {
            return Err(Error::new(
                def.fields.span(),
                "`Props` requires named fields or a unit struct",
            ));
        }

        let mut required_fields = Vec::with_capacity(def.fields.len());
        for field in &def.fields {
            if field.ident.as_ref().is_some_and(|ident| ident == "key") {
                return Err(Error::new(
                    field.span(),
                    "the `key` property name is reserved",
                ));
            }

            let mut required = false;
            for attr in &field.attrs {
                if !attr.path().is_ident("iocraft") {
                    continue;
                }
                attr.parse_nested_meta(|meta| {
                    if !meta.path.is_ident("required") {
                        return Err(meta.error("unsupported `iocraft` property attribute"));
                    }
                    if required {
                        return Err(meta.error("duplicate `required` attribute"));
                    }
                    if !meta.input.is_empty() {
                        return Err(meta.error("`required` does not accept a value"));
                    }
                    required = true;
                    Ok(())
                })?;
            }
            required_fields.push(required);
        }

        Ok(Self {
            def,
            required_fields,
        })
    }
}

impl ToTokens for ParsedProps {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let def = &self.def;
        let name = &def.ident;

        let has_generics = !def.generics.params.is_empty();
        let lifetime_generic_count = def
            .generics
            .params
            .iter()
            .filter(|param| matches!(param, GenericParam::Lifetime(_)))
            .count();

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

        let vis = &def.vis;
        let builder_name = format_ident!("__Iocraft{}Builder", name);
        let fields = def.fields.iter().collect::<Vec<_>>();
        let field_names = fields
            .iter()
            .map(|field| {
                field
                    .ident
                    .as_ref()
                    .expect("named fields checked while parsing")
            })
            .collect::<Vec<_>>();
        let required_fields = fields
            .iter()
            .zip(&self.required_fields)
            .filter_map(|(field, required)| required.then_some(*field))
            .collect::<Vec<_>>();
        let required_state_names = required_fields
            .iter()
            .enumerate()
            .map(|(index, _)| format_ident!("__IocraftRequired{index}"))
            .collect::<Vec<_>>();
        let missing_state_names = required_fields
            .iter()
            .map(|field| {
                let field_name = field
                    .ident
                    .as_ref()
                    .expect("named fields checked while parsing")
                    .to_string();
                format_ident!(
                    "__Iocraft{}MissingRequiredProperty_{}",
                    name,
                    field_name.trim_start_matches("r#")
                )
            })
            .collect::<Vec<_>>();
        let original_generic_args = def
            .generics
            .params
            .iter()
            .map(|param| match param {
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
            })
            .collect::<Vec<_>>();

        let mut builder_generics = def.generics.clone();
        for state_name in &required_state_names {
            builder_generics.params.push(parse_quote!(#state_name));
        }
        let (builder_impl_generics, builder_ty_generics, builder_where_clause) =
            builder_generics.split_for_impl();

        let builder_fields = fields
            .iter()
            .zip(&self.required_fields)
            .scan(0, |required_index, (field, required)| {
                let name = field
                    .ident
                    .as_ref()
                    .expect("named fields checked while parsing");
                let ty = &field.ty;
                let field_ty = if *required {
                    let state_name = &required_state_names[*required_index];
                    *required_index += 1;
                    quote!(#state_name)
                } else {
                    quote!(#ty)
                };
                Some(quote!(#name: #field_ty))
            })
            .collect::<Vec<_>>();

        let initial_builder_args = original_generic_args
            .iter()
            .cloned()
            .chain(missing_state_names.iter().map(|name| quote!(#name)))
            .collect::<Vec<_>>();
        let initial_builder_type = if initial_builder_args.is_empty() {
            quote!(#builder_name)
        } else {
            quote!(#builder_name<#(#initial_builder_args),*>)
        };
        let initial_field_values = field_names.iter().zip(&self.required_fields).scan(
            0,
            |required_index, (name, required)| {
                if *required {
                    let missing_state_name = &missing_state_names[*required_index];
                    *required_index += 1;
                    Some(quote!(#name: #missing_state_name))
                } else {
                    Some(quote!(#name: Default::default()))
                }
            },
        );

        let mut constructor_generics = def.generics.clone();
        for (field, required) in fields.iter().zip(&self.required_fields) {
            if !required {
                let ty = &field.ty;
                constructor_generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote!(#ty: Default));
            }
        }
        let (constructor_impl_generics, constructor_ty_generics, constructor_where_clause) =
            constructor_generics.split_for_impl();

        let optional_setters =
            fields
                .iter()
                .zip(&self.required_fields)
                .filter_map(|(field, required)| {
                    if *required {
                        return None;
                    }
                    let name = field
                        .ident
                        .as_ref()
                        .expect("named fields checked while parsing");
                    let ty = &field.ty;
                    Some(quote! {
                        #vis fn #name(mut self, value: impl Into<#ty>) -> Self {
                            self.#name = value.into();
                            self
                        }
                    })
                });

        let required_setters = required_fields
            .iter()
            .enumerate()
            .map(|(current_index, field)| {
                let field_name = field
                    .ident
                    .as_ref()
                    .expect("named fields checked while parsing");
                let field_ty = &field.ty;
                let return_args = original_generic_args
                    .iter()
                    .cloned()
                    .chain(required_fields.iter().enumerate().map(|(index, field)| {
                        if index == current_index {
                            let ty = &field.ty;
                            quote!(::iocraft::RequiredPropSet<#ty>)
                        } else {
                            let state_name = &required_state_names[index];
                            quote!(#state_name)
                        }
                    }))
                    .collect::<Vec<_>>();
                let return_type = quote!(#builder_name<#(#return_args),*>);
                let moved_fields = field_names.iter().map(|name| {
                    if *name == field_name {
                        quote!(#name: ::iocraft::RequiredPropSet(value.into()))
                    } else {
                        quote!(#name: self.#name)
                    }
                });
                quote! {
                    #vis fn #field_name(
                        self,
                        value: impl Into<#field_ty>,
                    ) -> #return_type {
                        #builder_name {
                            #(#moved_fields,)*
                            _iocraft_marker: std::marker::PhantomData,
                        }
                    }
                }
            });
        let complete_builder_args = original_generic_args
            .iter()
            .cloned()
            .chain(required_fields.iter().map(|field| {
                let ty = &field.ty;
                quote!(::iocraft::RequiredPropSet<#ty>)
            }))
            .collect::<Vec<_>>();
        let complete_builder_type = if complete_builder_args.is_empty() {
            quote!(#builder_name)
        } else {
            quote!(#builder_name<#(#complete_builder_args),*>)
        };

        let completed_field_values =
            field_names
                .iter()
                .zip(&self.required_fields)
                .map(|(name, required)| {
                    if *required {
                        quote!(#name: self.#name.0)
                    } else {
                        quote!(#name: self.#name)
                    }
                });
        let missing_state_markers = missing_state_names.iter().map(|name| {
            quote! {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                #vis struct #name;
            }
        });
        let props_value = match &def.fields {
            Fields::Unit => quote!(#name),
            Fields::Named(_) => quote!(#name {
                #(#completed_field_values,)*
            }),
            Fields::Unnamed(_) => unreachable!("tuple structs rejected while parsing"),
        };
        let (props_impl_generics, _, props_where_clause) = def.generics.split_for_impl();

        tokens.extend(quote! {
            #(#missing_state_markers)*
            #[doc(hidden)]
            #vis struct #builder_name #builder_impl_generics #builder_where_clause {
                #(#builder_fields,)*
                _iocraft_marker: std::marker::PhantomData<#name #bracketed_generic_names>,
            }

            impl #constructor_impl_generics #name #constructor_ty_generics
                #constructor_where_clause
            {
                #[doc(hidden)]
                #vis fn __iocraft_builder() -> #initial_builder_type {
                    #builder_name {
                        #(#initial_field_values,)*
                        _iocraft_marker: std::marker::PhantomData,
                    }
                }
            }

            impl #builder_impl_generics #builder_name #builder_ty_generics #builder_where_clause {
                #(#optional_setters)*
                #(#required_setters)*
            }

            impl #props_impl_generics #complete_builder_type #props_where_clause {
                #[doc(hidden)]
                #vis fn __iocraft_build(self) -> #name #bracketed_generic_names {
                    #props_value
                }
            }
        });

        tokens.extend(quote! {
            unsafe impl #props_impl_generics ::iocraft::Props for
                #name #bracketed_generic_names #props_where_clause
            {}
        });
    }
}

/// Makes a struct available for use as component properties.
///
/// Most importantly, this marks a struct as being
/// [covariant](https://doc.rust-lang.org/nomicon/subtyping.html). If the struct is not actually
/// covariant, compilation will fail.
///
/// Fields are initialized with their [`Default`] value unless they are marked as required:
///
/// ```
/// # use iocraft::prelude::*;
/// struct RequiredValue;
///
/// #[derive(Props)]
/// struct MyProps {
///     optional: String,
///     #[iocraft(required)]
///     required: RequiredValue,
/// }
///
/// #[component]
/// fn MyComponent(_props: &MyProps) -> impl Into<AnyElement<'static>> {
///     element!(View)
/// }
///
/// let _: Element<MyComponent> = element!(MyComponent(required: RequiredValue));
/// ```
///
/// Omitting a required property fails at compile time:
///
/// ```compile_fail
/// # use iocraft::prelude::*;
/// # struct RequiredValue;
/// # #[derive(Props)]
/// # struct MyProps {
/// #     #[iocraft(required)]
/// #     required: RequiredValue,
/// # }
/// # #[component]
/// # fn MyComponent(_props: &MyProps) -> impl Into<AnyElement<'static>> {
/// #     element!(View)
/// # }
/// let _ = element!(MyComponent);
/// ```
///
/// Compilation will also fail if the struct contains a "key" field, as that name is reserved to
/// facilitate tracking state across renders:
///
/// ```compile_fail
/// # use iocraft::prelude::*;
/// #[derive(Default, Props)]
/// struct MyProps {
///     key: i32,
/// }
/// ```
#[proc_macro_derive(Props, attributes(iocraft))]
pub fn derive_props(item: TokenStream) -> TokenStream {
    let props = parse_macro_input!(item as ParsedProps);
    quote!(#props).into()
}

struct ParsedComponent {
    f: ItemFn,
    props_type: Option<Box<Type>>,
    impl_args: Vec<proc_macro2::TokenStream>,
}

impl Parse for ParsedComponent {
    fn parse(input: ParseStream) -> Result<Self> {
        let f: ItemFn = input.parse()?;

        let mut props_type = None;
        let mut impl_args = Vec::new();

        for arg in &f.sig.inputs {
            match arg {
                FnArg::Typed(arg) => {
                    let name = match &*arg.pat {
                        Pat::Ident(arg) => arg.ident.to_string(),
                        _ => return Err(Error::new(arg.pat.span(), "invalid argument")),
                    };

                    match name.as_str() {
                        "props" | "_props" => {
                            if props_type.is_some() {
                                return Err(Error::new(arg.span(), "duplicate `props` argument"));
                            }
                            match &*arg.ty {
                                Type::Reference(r) => {
                                    props_type = Some(r.elem.clone());
                                    impl_args.push(quote!(props));
                                }
                                _ => {
                                    return Err(Error::new(
                                        arg.ty.span(),
                                        "invalid `props` type (must be a reference)",
                                    ))
                                }
                            }
                        }
                        "hooks" | "_hooks" => match &*arg.ty {
                            Type::Reference(_) => {
                                impl_args.push(quote!(&mut hooks));
                            }
                            Type::Path(_) => {
                                impl_args.push(quote!(hooks));
                            }
                            _ => {
                                return Err(Error::new(
                                    arg.ty.span(),
                                    "invalid `hooks` type (must be a reference or a value)",
                                ))
                            }
                        },
                        _ => {
                            return Err(Error::new(
                                arg.span(),
                                "unexpected argument (must be named `props` or `hooks`)",
                            ))
                        }
                    }
                }
                _ => return Err(Error::new(arg.span(), "invalid argument")),
            }
        }

        Ok(Self {
            f,
            props_type,
            impl_args,
        })
    }
}

impl ToTokens for ParsedComponent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let attrs = &self.f.attrs;
        let vis = &self.f.vis;
        let name = &self.f.sig.ident;
        let args = &self.f.sig.inputs;
        let block = &self.f.block;
        let output = &self.f.sig.output;
        let generics = &self.f.sig.generics;
        let lifetime_generics = {
            Generics {
                params: generics
                    .params
                    .iter()
                    .filter(|param| matches!(param, GenericParam::Lifetime(_)))
                    .cloned()
                    .collect(),
                where_clause: generics
                    .where_clause
                    .as_ref()
                    .map(|where_clause| WhereClause {
                        where_token: where_clause.where_token,
                        predicates: where_clause
                            .predicates
                            .iter()
                            .filter(|predicate| matches!(predicate, WherePredicate::Lifetime(_)))
                            .cloned()
                            .collect(),
                    }),
                ..generics.clone()
            }
        };
        let (lifetime_impl_generics, _lifetime_ty_generics, lifetime_where_clause) =
            lifetime_generics.split_for_impl();
        let type_generics = {
            Generics {
                params: generics
                    .params
                    .iter()
                    .filter(|param| !matches!(param, GenericParam::Lifetime(_)))
                    .cloned()
                    .collect(),
                where_clause: generics
                    .where_clause
                    .as_ref()
                    .map(|where_clause| WhereClause {
                        where_token: where_clause.where_token,
                        predicates: where_clause
                            .predicates
                            .iter()
                            .filter(|predicate| !matches!(predicate, WherePredicate::Lifetime(_)))
                            .cloned()
                            .collect(),
                    }),
                ..generics.clone()
            }
        };
        let (impl_generics, ty_generics, where_clause) = type_generics.split_for_impl();
        let ty_generic_names = type_generics.params.iter().filter_map(|param| match param {
            GenericParam::Type(ty) => {
                let name = &ty.ident;
                Some(quote!(#name))
            }
            _ => None,
        });
        let impl_args = &self.impl_args;

        let props_type_name = self
            .props_type
            .as_ref()
            .map(|ty| quote!(#ty))
            .unwrap_or_else(|| quote!(::iocraft::NoProps));

        tokens.extend(quote! {
            #(#attrs)*
            #vis struct #name #impl_generics {
                _marker: std::marker::PhantomData<fn(#(#ty_generic_names),*)>,
            }

            impl #impl_generics #name #ty_generics #where_clause {
                fn implementation #lifetime_impl_generics (#args) #output #lifetime_where_clause #block
            }

            impl #impl_generics ::iocraft::Component for #name #ty_generics #where_clause {
                type Props<'a> = #props_type_name;

                fn new(_props: &Self::Props<'_>) -> Self {
                    Self{
                        _marker: std::marker::PhantomData,
                    }
                }

                fn update(&mut self, props: &mut Self::Props<'_>, mut hooks: ::iocraft::Hooks, updater: &mut ::iocraft::ComponentUpdater) {
                    let mut e = {
                        let mut hooks = hooks.with_context_stack(updater.component_context_stack());
                        Self::implementation(#(#impl_args),*).into()
                    };
                    updater.set_transparent_layout(true);
                    updater.update_children([&mut e], None);
                }
            }
        });
    }
}

/// Defines a custom component type.
///
/// Custom components are defined by adding this macro to a function that returns an `Element`:
///
/// ```
/// # use iocraft::prelude::*;
/// # use std::time::Duration;
/// #[component]
/// fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let mut count = hooks.use_state(|| 0);
///
///     hooks.use_future(async move {
///         loop {
///             smol::Timer::after(Duration::from_millis(100)).await;
///             count += 1;
///         }
///     });
///
///     element! {
///         Text(color: Color::Blue, content: format!("counter: {}", count))
///     }
/// }
/// ```
///
/// The function is allowed to take up to two arguments, one named `props`, for the component's
/// properties and one named `hooks`, for hooks.
///
/// Here is an example of a component that takes a reference to a `Vec` of `User` structs via properties:
///
/// ```
/// # use iocraft::prelude::*;
/// # struct User {
/// #     id: i32,
/// #     name: String,
/// #     email: String,
/// # }
/// #[derive(Default, Props)]
/// struct UsersTableProps<'a> {
///     users: Option<&'a Vec<User>>,
/// }
///
/// #[component]
/// fn UsersTable<'a>(props: &UsersTableProps<'a>) -> impl Into<AnyElement<'a>> {
///     element! {
///         View(
///             margin_top: 1,
///             margin_bottom: 1,
///             flex_direction: FlexDirection::Column,
///             width: 60,
///             border_style: BorderStyle::Round,
///             border_color: Color::Cyan,
///         ) {
///             View(border_style: BorderStyle::Single, border_edges: Edges::Bottom, border_color: Color::Grey) {
///                 View(width: 10pct, justify_content: JustifyContent::End, padding_right: 2) {
///                     Text(content: "Id", weight: Weight::Bold, decoration: TextDecoration::Underline)
///                 }
///
///                 View(width: 40pct) {
///                     Text(content: "Name", weight: Weight::Bold, decoration: TextDecoration::Underline)
///                 }
///
///                 View(width: 50pct) {
///                     Text(content: "Email", weight: Weight::Bold, decoration: TextDecoration::Underline)
///                 }
///             }
///
///             #(props.users.map(|users| users.iter().enumerate().map(|(i, user)| element! {
///                 View(background_color: if i % 2 == 0 { None } else { Some(Color::DarkGrey) }) {
///                     View(width: 10pct, justify_content: JustifyContent::End, padding_right: 2) {
///                         Text(content: user.id.to_string())
///                     }
///
///                     View(width: 40pct) {
///                         Text(content: user.name.clone())
///                     }
///
///                     View(width: 50pct) {
///                         Text(content: user.email.clone())
///                     }
///                 }
///             })).into_iter().flatten())
///         }
///     }
/// }
/// ```
///
/// Components can also be generic:
///
/// ```
/// # use iocraft::prelude::*;
/// #[derive(Default, Props)]
/// struct MyGenericComponentProps<T: Send + Sync> {
///     items: Vec<T>,
/// }
///
/// #[component]
/// fn MyGenericComponent<T: Send + Sync + 'static>(
///     _props: &MyGenericComponentProps<T>,
/// ) -> impl Into<AnyElement<'static>> {
///     element!(View)
/// }
/// ```
///
/// However, note that generic type parameters must be `'static`.
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let component = parse_macro_input!(item as ParsedComponent);
    quote!(#component).into()
}

#[doc(hidden)]
#[proc_macro_attribute]
pub fn with_layout_style_props(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let layout_style_fields = [
        quote! {
            /// Sets the display mode for the element. Defaults to [`Display::Flex`].
            ///
            /// See [the MDN documentation for display](https://developer.mozilla.org/en-US/docs/Web/CSS/display).
            pub display: ::iocraft::Display
        },
        quote! {
            /// Sets the width of the element.
            pub width: ::iocraft::Size
        },
        quote! {
            /// Sets the height of the element.
            pub height: ::iocraft::Size
        },
        quote! {
            /// Sets the minimum width of the element.
            pub min_width: ::iocraft::Size
        },
        quote! {
            /// Sets the minimum height of the element.
            pub min_height: ::iocraft::Size
        },
        quote! {
            /// Sets the maximum width of the element.
            pub max_width: ::iocraft::Size
        },
        quote! {
            /// Sets the maximum height of the element.
            pub max_height: ::iocraft::Size
        },
        quote! {
            /// Defines the gaps in between rows or columns of flex items.
            ///
            /// See [the MDN documentation for gap](https://developer.mozilla.org/en-US/docs/Web/CSS/gap).
            pub gap: ::iocraft::Gap
        },
        quote! {
            /// Defines the gaps in between columns of flex items.
            ///
            /// See [the MDN documentation for column-gap](https://developer.mozilla.org/en-US/docs/Web/CSS/column-gap).
            pub column_gap: ::iocraft::Gap
        },
        quote! {
            /// Defines the gaps in between rows of flex items.
            ///
            /// See [the MDN documentation for row-gap](https://developer.mozilla.org/en-US/docs/Web/CSS/row-gap).
            pub row_gap: ::iocraft::Gap
        },
        quote! {
            /// Defines the area to reserve around the element's content, but inside the border.
            ///
            /// See [the MDN documentation for padding](https://developer.mozilla.org/en-US/docs/Web/CSS/padding).
            pub padding: ::iocraft::Padding
        },
        quote! {
            /// Defines the area to reserve above the element's content, but inside the border.
            ///
            /// See [the MDN documentation for padding](https://developer.mozilla.org/en-US/docs/Web/CSS/padding).
            pub padding_top: ::iocraft::Padding
        },
        quote! {
            /// Defines the area to reserve to the right of the element's content, but inside the border.
            ///
            /// See [the MDN documentation for padding](https://developer.mozilla.org/en-US/docs/Web/CSS/padding).
            pub padding_right: ::iocraft::Padding
        },
        quote! {
            /// Defines the area to reserve below the element's content, but inside the border.
            ///
            /// See [the MDN documentation for padding](https://developer.mozilla.org/en-US/docs/Web/CSS/padding).
            pub padding_bottom: ::iocraft::Padding
        },
        quote! {
            /// Defines the area to reserve to the left of the element's content, but inside the border.
            ///
            /// See [the MDN documentation for padding](https://developer.mozilla.org/en-US/docs/Web/CSS/padding).
            pub padding_left: ::iocraft::Padding
        },
        quote! {
            /// Controls how the element is layed out and whether it will be controlled by the flexbox.
            pub position: ::iocraft::Position
        },
        quote! {
            /// Sets the position of a positioned element.
            ///
            /// See [the MDN documentation for inset](https://developer.mozilla.org/en-US/docs/Web/CSS/inset).
            pub inset: ::iocraft::Inset
        },
        quote! {
            /// Sets the vertical position of a positioned element.
            ///
            /// See [the MDN documentation for top](https://developer.mozilla.org/en-US/docs/Web/CSS/top).
            pub top: ::iocraft::Inset
        },
        quote! {
            /// Sets the horizontal position of a positioned element.
            ///
            /// See [the MDN documentation for right](https://developer.mozilla.org/en-US/docs/Web/CSS/right).
            pub right: ::iocraft::Inset
        },
        quote! {
            /// Sets the vertical position of a positioned element.
            ///
            /// See [the MDN documentation for bottom](https://developer.mozilla.org/en-US/docs/Web/CSS/bottom).
            pub bottom: ::iocraft::Inset
        },
        quote! {
            /// Sets the horizontal position of a positioned element.
            ///
            /// See [the MDN documentation for left](https://developer.mozilla.org/en-US/docs/Web/CSS/left).
            pub left: ::iocraft::Inset
        },
        quote! {
            /// Defines the area to reserve around the element's content, but outside the border.
            ///
            /// See [the MDN documentation for margin](https://developer.mozilla.org/en-US/docs/Web/CSS/margin).
            pub margin: ::iocraft::Margin
        },
        quote! {
            /// Defines the area to reserve above the element's content, but outside the border.
            ///
            /// See [the MDN documentation for margin](https://developer.mozilla.org/en-US/docs/Web/CSS/margin).
            pub margin_top: ::iocraft::Margin
        },
        quote! {
            /// Defines the area to reserve to the right of the element's content, but outside the border.
            ///
            /// See [the MDN documentation for margin](https://developer.mozilla.org/en-US/docs/Web/CSS/margin).
            pub margin_right: ::iocraft::Margin
        },
        quote! {
            /// Defines the area to reserve below the element's content, but outside the border.
            ///
            /// See [the MDN documentation for margin](https://developer.mozilla.org/en-US/docs/Web/CSS/margin).
            pub margin_bottom: ::iocraft::Margin
        },
        quote! {
            /// Defines the area to reserve to the left of the element's content, but outside the border.
            ///
            /// See [the MDN documentation for margin](https://developer.mozilla.org/en-US/docs/Web/CSS/margin).
            pub margin_left: ::iocraft::Margin
        },
        quote! {
            /// Defines how items are placed along the main axis of a flex container.
            ///
            /// See [the MDN documentation for flex-direction](https://developer.mozilla.org/en-US/docs/Web/CSS/flex-direction).
            pub flex_direction: ::iocraft::FlexDirection
        },
        quote! {
            /// Defines whether items are forced onto one line or can wrap into multiple lines.
            ///
            /// See [the MDN documentation for flex-wrap](https://developer.mozilla.org/en-US/docs/Web/CSS/flex-wrap).
            pub flex_wrap: ::iocraft::FlexWrap
        },
        quote! {
            /// Sets the initial main size of a flex item.
            ///
            /// See [the MDN documentation for flex-basis](https://developer.mozilla.org/en-US/docs/Web/CSS/flex-basis).
            pub flex_basis: ::iocraft::FlexBasis
        },
        quote! {
            /// Sets the flex grow factor, which specifies how much free space should be assigned
            /// to the item's main size.
            ///
            /// See [the MDN documentation for flex-grow](https://developer.mozilla.org/en-US/docs/Web/CSS/flex-grow).
            pub flex_grow: f32
        },
        quote! {
            /// Sets the flex shrink factor, which specifies how the item should shrink when the
            /// container doesn't have enough room for all flex items.
            ///
            /// See [the MDN documentation for flex-shrink](https://developer.mozilla.org/en-US/docs/Web/CSS/flex-shrink).
            pub flex_shrink: Option<f32>
        },
        quote! {
            /// Controls the alignment of items along the cross axis of a flex container.
            ///
            /// See [the MDN documentation for align-items](https://developer.mozilla.org/en-US/docs/Web/CSS/align-items).
            pub align_items: Option<::iocraft::AlignItems>
        },
        quote! {
            /// Controls the distribution of space between and around items along a flex container's cross axis.
            ///
            /// See [the MDN documentation for align-content](https://developer.mozilla.org/en-US/docs/Web/CSS/align-content).
            pub align_content: Option<::iocraft::AlignContent>
        },
        quote! {
            /// Controls the distribution of space between and around items along a flex container's main axis.
            ///
            /// See [the MDN documentation for justify-content](https://developer.mozilla.org/en-US/docs/Web/CSS/justify-content).
            pub justify_content: Option<::iocraft::JustifyContent>
        },
        quote! {
            /// Defines the behavior when content does not fit within the element's padding box.
            ///
            /// See [the MDN documentation for overflow](https://developer.mozilla.org/en-US/docs/Web/CSS/overflow).
            pub overflow: Option<::iocraft::Overflow>
        },
        quote! {
            /// Defines the behavior when content does not fit within the element's padding box in the horizontal direction.
            ///
            /// See [the MDN documentation for overflow](https://developer.mozilla.org/en-US/docs/Web/CSS/overflow).
            pub overflow_x: Option<::iocraft::Overflow>
        },
        quote! {
            /// Defines the behavior when content does not fit within the element's padding box in the vertical direction.
            ///
            /// See [the MDN documentation for overflow](https://developer.mozilla.org/en-US/docs/Web/CSS/overflow).
            pub overflow_y: Option<::iocraft::Overflow>
        },
    ]
    .map(|tokens| syn::Field::parse_named.parse2(tokens).unwrap());

    let mut ast = parse_macro_input!(item as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            if let syn::Fields::Named(fields) = &mut struct_data.fields {
                fields.named.extend(layout_style_fields.iter().cloned());
            }

            let struct_name = &ast.ident;
            let field_assignments = layout_style_fields.iter().map(|field| {
                let field_name = &field.ident;
                quote! { ret.#field_name = self.#field_name; }
            });

            let where_clause = &ast.generics.where_clause;

            let has_generics = !ast.generics.params.is_empty();
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

            quote! {
                #ast

                impl #generics #struct_name #bracketed_generic_names #where_clause {
                    /// Returns the layout style based on the layout-related fields of this struct.
                    pub fn layout_style(&self) -> ::iocraft::LayoutStyle {
                        let mut ret: ::iocraft::LayoutStyle = Default::default();
                        #(#field_assignments)*
                        ret
                    }
                }
            }
            .into()
        }
        _ => panic!("`with_layout_style_props` can only be used with structs "),
    }
}
