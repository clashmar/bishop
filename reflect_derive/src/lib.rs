// reflect_derive/src/lib.rs
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::LitStr;
use syn::Token;
use syn::parse_macro_input;

/// `#[derive(Reflect)]` – generates an impl of the `Reflect` trait.
#[proc_macro_derive(Reflect, attributes(widget))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();
    let generics = input.generics;

    let field_infos = match input.data {
        Data::Struct(s) => match s.fields {
            // Named fields: struct Foo { x: i32, y: f32 }
            Fields::Named(named) => {
                let infos = named.named.iter().map(|f| {
                    let field_name = f.ident.as_ref().unwrap();
                    let field_str = field_name.to_string();
                    let ty = &f.ty;

                    let hint_opt = widget_hint(&f.attrs);
                    let hint_expr = match hint_opt {
                        Some(s) => quote! { Some(#s) },
                        None => quote! { None },
                    };

                    quote! {
                        <#ty as crate::ecs::reflect_field::ReflectField>::field_info(
                            &mut self.#field_name,
                            #field_str
                        )
                        .with_hint(#hint_expr)
                    }
                });
                quote! { vec![#(#infos),*] }
            }
            // Tuple struct: struct Foo(i32) or struct Foo(i32, f32)
            Fields::Unnamed(unnamed) => {
                if unnamed.unnamed.len() == 1 {
                    // Single field tuple struct (newtype pattern)
                    let field = unnamed.unnamed.first().unwrap();
                    let ty = &field.ty;

                    // Use the struct name as the field name for single-field tuples
                    let field_name = name.to_string();

                    let hint_opt = widget_hint(&field.attrs);
                    let hint_expr = match hint_opt {
                        Some(s) => quote! { Some(#s) },
                        None => quote! { None },
                    };

                    quote! {
                        vec![
                            <#ty as crate::ecs::reflect_field::ReflectField>::field_info(
                                &mut self.0,
                                #field_name
                            )
                            .with_hint(#hint_expr)
                        ]
                    }
                } else {
                    // Multiple field tuple struct
                    let infos = unnamed.unnamed.iter().enumerate().map(|(idx, f)| {
                        let ty = &f.ty;
                        let field_name = format!("field_{}", idx);
                        let index = syn::Index::from(idx);

                        let hint_opt = widget_hint(&f.attrs);
                        let hint_expr = match hint_opt {
                            Some(s) => quote! { Some(#s) },
                            None => quote! { None },
                        };

                        quote! {
                            <#ty as crate::ecs::reflect_field::ReflectField>::field_info(
                                &mut self.#index,
                                #field_name
                            )
                            .with_hint(#hint_expr)
                        }
                    });
                    quote! { vec![#(#infos),*] }
                }
            }
            // Unit struct: struct Foo;
            Fields::Unit => {
                quote! { vec![] }
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Reflect can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics crate::ecs::reflect_field::Reflect for #name #ty_generics #where_clause {
            fn fields(&mut self) -> ::std::vec::Vec<crate::ecs::reflect_field::FieldInfo<'_>> {
                #field_infos
            }
        }
    };

    TokenStream::from(expanded)
}

/// Returns the string literal that appears in `#[widget = "…"]`.
/// If the attribute is missing, has a different name, or the value is not a
/// string literal, `None` is returned.
fn widget_hint(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("widget") {
            continue;
        }

        // Parse the token stream after the path
        let parser = |input: syn::parse::ParseStream| {
            if input.peek(Token![=]) {
                let _eq: Token![=] = input.parse()?;
            }
            let lit: LitStr = input.parse()?;
            Ok(lit)
        };

        // If parsing fails we just ignore the attribute
        if let Ok(lit) = attr.parse_args_with(parser) {
            return Some(lit.value());
        }
    }
    None
}
