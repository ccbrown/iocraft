//! This crate defines the macros used by `iocraft`.

#![warn(missing_docs)]

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
    Lit, Member, Pat, Result, Token, Type, TypePath,
};
use uuid::Uuid;

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

        let decl_key = Uuid::new_v4().as_u128();

        let key = self
            .props
            .iter()
            .find_map(|FieldValue { member, expr, .. }| match member {
                Member::Named(ident) if ident == "key" => Some(quote!((#decl_key, #expr))),
                _ => None,
            })
            .unwrap_or_else(|| quote!(#decl_key));

        let prop_assignments = self
            .props
            .iter()
            .filter_map(|FieldValue { member, expr, .. }| match member {
                Member::Named(ident) if ident == "key" => None,
                _ => Some(match expr {
                    Expr::Lit(lit) => match &lit.lit {
                        Lit::Int(lit) if lit.suffix() == "pct" => {
                            let value = lit.base10_parse::<f32>().unwrap();
                            quote!(_iocraft_props.#member = ::iocraft::Percent(#value).into())
                        }
                        Lit::Float(lit) if lit.suffix() == "pct" => {
                            let value = lit.base10_parse::<f32>().unwrap();
                            quote!(_iocraft_props.#member = ::iocraft::Percent(#value).into())
                        }
                        _ => quote!(_iocraft_props.#member = (#expr).into()),
                    },
                    _ => quote!(_iocraft_props.#member = (#expr).into()),
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
                let mut _iocraft_props: Props = Default::default();
                #(#prop_assignments;)*
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

// This is documented in the `iocraft` crate instead so that links to `iocraft` types resolve correctly.
#[allow(missing_docs)]
#[proc_macro]
pub fn element(input: TokenStream) -> TokenStream {
    let element = parse_macro_input!(input as ParsedElement);
    quote!(#element).into()
}

struct ParsedProps {
    def: ItemStruct,
}

impl Parse for ParsedProps {
    fn parse(input: ParseStream) -> Result<Self> {
        let def: ItemStruct = input.parse()?;
        Ok(Self { def })
    }
}

impl ToTokens for ParsedProps {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let def = &self.def;
        let name = &def.ident;
        let where_clause = &def.generics.where_clause;

        let has_generics = !def.generics.params.is_empty();
        let lifetime_generic_count = def
            .generics
            .params
            .iter()
            .filter(|param| matches!(param, GenericParam::Lifetime(_)))
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
            unsafe impl #generics ::iocraft::Props for #name #bracketed_generic_names #where_clause {}
        });
    }
}

/// Makes a struct available for use as component properties.
///
/// Most importantly, this marks a struct as being
/// [covariant](https://doc.rust-lang.org/nomicon/subtyping.html). If the struct is not actually
/// covariant, compilation will fail.
#[proc_macro_derive(Props)]
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
        let vis = &self.f.vis;
        let name = &self.f.sig.ident;
        let args = &self.f.sig.inputs;
        let block = &self.f.block;
        let output = &self.f.sig.output;
        let generics = &self.f.sig.generics;
        let impl_args = &self.impl_args;

        let props_type_name = self
            .props_type
            .as_ref()
            .map(|ty| quote!(#ty))
            .unwrap_or_else(|| quote!(::iocraft::NoProps));

        tokens.extend(quote! {
            #vis struct #name;

            impl #name {
                fn implementation #generics (#args) #output #block
            }

            impl ::iocraft::Component for #name {
                type Props<'a> = #props_type_name;

                fn new(_props: &Self::Props<'_>) -> Self {
                    Self
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
///         Box(
///             margin_top: 1,
///             margin_bottom: 1,
///             flex_direction: FlexDirection::Column,
///             width: 60,
///             border_style: BorderStyle::Round,
///             border_color: Color::Cyan,
///         ) {
///             Box(border_style: BorderStyle::Single, border_edges: Edges::Bottom, border_color: Color::Grey) {
///                 Box(width: 10pct, justify_content: JustifyContent::End, padding_right: 2) {
///                     Text(content: "Id", weight: Weight::Bold, decoration: TextDecoration::Underline)
///                 }
///
///                 Box(width: 40pct) {
///                     Text(content: "Name", weight: Weight::Bold, decoration: TextDecoration::Underline)
///                 }
///
///                 Box(width: 50pct) {
///                     Text(content: "Email", weight: Weight::Bold, decoration: TextDecoration::Underline)
///                 }
///             }
///
///             #(props.users.map(|users| users.iter().enumerate().map(|(i, user)| element! {
///                 Box(background_color: if i % 2 == 0 { None } else { Some(Color::DarkGrey) }) {
///                     Box(width: 10pct, justify_content: JustifyContent::End, padding_right: 2) {
///                         Text(content: user.id.to_string())
///                     }
///
///                     Box(width: 40pct) {
///                         Text(content: user.name.clone())
///                     }
///
///                     Box(width: 50pct) {
///                         Text(content: user.email.clone())
///                     }
///                 }
///             })).into_iter().flatten())
///         }
///     }
/// }
/// ```
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
                quote! { #field_name: self.#field_name }
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
                        ::iocraft::LayoutStyle{
                            #(#field_assignments,)*
                        }
                    }
                }
            }
            .into()
        }
        _ => panic!("`with_layout_style_props` can only be used with structs "),
    }
}
