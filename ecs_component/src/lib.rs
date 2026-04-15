// ecs_component/src/lib.rs
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::Path;
use syn::Token;
use syn::Type;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;

struct EcsComponentArgs {
    deps: Vec<Type>,
    post_create: Option<Path>,
    post_remove: Option<Path>,
}

impl Parse for EcsComponentArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut deps = Vec::new();
        let mut post_create = None;
        let mut post_remove = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;

            if ident == "deps" {
                let content;
                syn::bracketed!(content in input);
                let types: Punctuated<Type, Token![,]> =
                    content.parse_terminated(Type::parse, Token![,])?;
                deps = types.into_iter().collect();
            } else if ident == "post_create" {
                post_create = Some(input.parse()?);
            } else if ident == "post_remove" {
                post_remove = Some(input.parse()?);
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Expected 'deps', 'post_create' or 'post_remove'",
                ));
            }

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        Ok(EcsComponentArgs {
            deps,
            post_create,
            post_remove,
        })
    }
}

/// `#[ecs_component]` – generates Component impl, LuaSchema, and registry submission
#[proc_macro_attribute]
pub fn ecs_component(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = if args.is_empty() {
        EcsComponentArgs {
            deps: Vec::new(),
            post_create: None,
            post_remove: None,
        }
    } else {
        parse_macro_input!(args as EcsComponentArgs)
    };

    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let generics = &input.generics;

    // Extract the struct data
    let struct_data = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return syn::Error::new_spanned(name, "ecs_component only works on structs")
                .to_compile_error()
                .into();
        }
    };

    let fields = &struct_data.fields;
    let deps = &args.deps;

    // Build the struct definition
    let struct_def = match fields {
        Fields::Named(named) => {
            let fields = named.named.iter().map(|field| {
                let field_attrs = &field.attrs;
                let field_vis = &field.vis;
                let field_ident = &field.ident;
                let field_ty = &field.ty;

                quote! {
                    #(#field_attrs)*
                    #field_vis #field_ident: #field_ty,
                }
            });

            quote! {
                #(#attrs)*
                #vis struct #name #generics {
                    #(#fields)*
                }
            }
        }
        Fields::Unnamed(unnamed) => {
            let fields = unnamed.unnamed.iter().map(|field| {
                let field_attrs = &field.attrs;
                let field_vis = &field.vis;
                let field_ty = &field.ty;

                quote! {
                    #(#field_attrs)*
                    #field_vis #field_ty,
                }
            });

            quote! {
                #(#attrs)*
                #vis struct #name #generics(
                    #(#fields)*
                );
            }
        }
        Fields::Unit => {
            quote! {
                #(#attrs)* #vis struct #name #generics;
            }
        }
    };

    // Generate LuaSchema implementation
    let lua_schema = generate_lua_schema(fields);

    // Generate factory with dependencies
    let factory_deps = deps.iter().map(|dep| {
        quote! {
            world.get_store_mut::<#dep>()
                .insert(entity, <#dep>::default());
        }
    });

    // Generate post_create function
    let post_create_fn = if let Some(func) = &args.post_create {
        quote! {
            |any: &mut dyn std::any::Any, entity: &Entity, ctx: &mut dyn crate::game::EngineCtxMut| {
                let comp = any
                    .downcast_mut::<#name>()
                    .expect(concat!(
                        "post_create: Type mismatch for ",
                        stringify!(#name)
                    ));
                #func(comp, entity, ctx);
            }
        }
    } else {
        quote! {
            crate::ecs::component_registry::noop_post_create
        }
    };

    // Generate post_remove function
    let post_remove_fn = if let Some(func) = &args.post_remove {
        // The user‑provided function now expects (comp, entity, ctx)
        quote! {
            |any: &mut dyn std::any::Any, entity: &Entity, ctx: &mut dyn crate::game::EngineCtxMut| {
                let comp = any
                    .downcast_mut::<#name>()
                    .expect(concat!(
                        "post_remove: Type mismatch for ",
                        stringify!(#name)
                    ));
                #func(comp, entity, ctx);
            }
        }
    } else {
        quote! {
            crate::ecs::component_registry::noop_post_remove
        }
    };

    let to_lua_impl = generate_to_lua_impl(fields, name);
    let from_lua_impl = generate_from_lua_impl(fields, name);

    let expanded = quote! {
        #struct_def

        // Component trait implementation
        impl #generics crate::ecs::component::Component for #name #generics {
            fn store_mut(
                world: &mut crate::ecs::ecs::Ecs,
            ) -> &mut crate::ecs::component::ComponentStore<Self> {
                world.get_or_create_store::<Self>()
            }
            fn store(
                world: &crate::ecs::ecs::Ecs,
            ) -> &crate::ecs::component::ComponentStore<Self> {
                world.get_store::<Self>()
            }
        }

        // LuaSchema trait implementation
        impl #generics crate::ecs::component_registry::LuaSchema for #name #generics {
            fn lua_schema() -> &'static [(&'static str, &'static str)] {
                #lua_schema
            }
        }

        impl #name #generics
        where
            #name #generics: 'static + Clone,
        {
            pub const TYPE_NAME: &'static str = stringify!(#name);

            fn __factory(
                world: &mut crate::ecs::ecs::Ecs,
                entity: crate::ecs::entity::Entity,
            ) {
                world.get_store_mut::<#name>()
                    .insert(entity, <#name>::default());
                #(#factory_deps)*
            }

            fn __to_ron(store: &dyn std::any::Any) -> String {
                let concrete = store
                    .downcast_ref::<crate::ecs::component::ComponentStore<#name>>()
                    .expect("type mismatch in to_ron");
                ron::ser::to_string_pretty(concrete, ron::ser::PrettyConfig::default())
                    .expect("failed to serialize ComponentStore")
            }

            fn __from_ron(text: String) -> Box<dyn std::any::Any + Send + Sync> {
                let concrete: crate::ecs::component::ComponentStore<#name> =
                    ron::de::from_str(&text).expect("failed to deserialize ComponentStore");
                Box::new(concrete)
            }

            fn __to_ron_component(value: &dyn std::any::Any) -> String {
                let concrete = value
                    .downcast_ref::<#name>()
                    .expect("type mismatch in to_ron_component");
                ron::ser::to_string_pretty(concrete, ron::ser::PrettyConfig::default())
                    .expect("failed to serialize component")
            }

            fn __from_ron_component(text: String) -> Box<dyn std::any::Any> {
                let concrete: #name =
                    ron::de::from_str(&text).expect("failed to deserialize component");
                Box::new(concrete) as Box<dyn std::any::Any>
            }

            fn __to_lua(lua: &mlua::Lua, any: &dyn std::any::Any) -> mlua::Result<mlua::Value> {
                #to_lua_impl
            }

            fn __from_lua(lua: &mlua::Lua, value: mlua::Value) -> mlua::Result<Box<dyn std::any::Any>> {
                #from_lua_impl
            }
        }

        // Registry submission
        inventory::submit! {
            crate::ecs::component_registry::ComponentRegistry {
                type_name: <#name>::TYPE_NAME,
                type_id: std::any::TypeId::of::<
                    crate::ecs::component::ComponentStore<#name>
                >(),
                to_ron: <#name>::__to_ron,
                from_ron: <#name>::__from_ron,
                factory: <#name>::__factory,
                has: crate::ecs::component_registry::has_component::<#name>,
                remove: crate::ecs::component_registry::erase_from_store::<#name>,
                inserter: crate::ecs::component_registry::generic_inserter::<#name>,
                clone: |world: &crate::ecs::ecs::Ecs,
                         entity: crate::ecs::entity::Entity| {
                    let store_any = world
                        .stores
                        .get(&std::any::TypeId::of::<
                            crate::ecs::component::ComponentStore<#name>
                        >())
                        .expect("store missing despite has() == true");
                    let component = {
                        let store = store_any
                            .downcast_ref::<
                                crate::ecs::component::ComponentStore<#name>
                            >()
                            .expect("Type mismatch in store");
                        store
                            .get(entity)
                            .expect("has() returned true but component missing")
                            .clone()
                    };
                    Box::new(component) as Box<dyn std::any::Any>
                },
                to_ron_component: <#name>::__to_ron_component,
                from_ron_component: <#name>::__from_ron_component,
                to_lua: <#name>::__to_lua,
                from_lua: <#name>::__from_lua,
                lua_schema: <#name as crate::ecs::component_registry::LuaSchema>::lua_schema,
                post_create: #post_create_fn,
                post_remove: #post_remove_fn,
            }
        }
    };

    TokenStream::from(expanded)
}

fn generate_lua_schema(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        // Normal struct { a: T, b: U }
        Fields::Named(named) => {
            let field_schemas = named.named.iter().map(|f| {
                let name = f.ident.as_ref().unwrap().to_string();
                let ty = &f.ty;
                let lua_type = rust_type_to_lua(ty);
                quote! {
                    (#name, #lua_type)
                }
            });

            quote! {
                &[#(#field_schemas),*]
            }
        }

        // Tuple struct: struct Foo(T)
        Fields::Unnamed(unnamed) => {
            if unnamed.unnamed.len() == 1 {
                // Mark single-field tuple structs as aliases using the inner type directly
                let field = unnamed.unnamed.first().unwrap();
                let lua_type = rust_alias_type_to_lua(&field.ty);

                quote! {
                    &[("__alias__", #lua_type)]
                }
            } else {
                // Multi-field tuple structs: generate field_0, field_1, etc.
                let field_schemas = unnamed.unnamed.iter().enumerate().map(|(i, f)| {
                    let name = format!("field_{}", i);
                    let lua_type = rust_type_to_lua(&f.ty);
                    quote! {
                        (#name, #lua_type)
                    }
                });

                quote! {
                    &[#(#field_schemas),*]
                }
            }
        }

        // Unit struct: struct Marker;
        Fields::Unit => {
            quote! { &[] }
        }
    }
}

fn rust_type_to_lua(ty: &Type) -> &'static str {
    match ty {
        syn::Type::Path(p)
            if p.path.is_ident("f32")
                || p.path.is_ident("f64")
                || p.path.is_ident("i8")
                || p.path.is_ident("i16")
                || p.path.is_ident("i32")
                || p.path.is_ident("i64")
                || p.path.is_ident("u8")
                || p.path.is_ident("u16")
                || p.path.is_ident("u32")
                || p.path.is_ident("u64")
                || p.path.is_ident("usize")
                || p.path.is_ident("isize") =>
        {
            "number"
        }
        // Bools
        syn::Type::Path(p) if p.path.is_ident("bool") => "boolean",
        // Strings
        syn::Type::Path(p) if p.path.is_ident("String") => "string",
        syn::Type::Reference(r)
            if matches!(r.elem.as_ref(),
                syn::Type::Path(p) if p.path.is_ident("str")) =>
        {
            "string"
        }
        // Math / engine primitives
        syn::Type::Path(p) if p.path.is_ident("Vec2") => "vec2",
        syn::Type::Path(p) if p.path.is_ident("Vec3") => "vec3",
        // Id types RoomId, SpriteId, etc.
        syn::Type::Path(p) => {
            let ident = p.path.segments.last().unwrap().ident.to_string();
            if ident.ends_with("Id") {
                "number"
            } else {
                "table"
            }
        }
        _ => "table",
    }
}

fn rust_alias_type_to_lua(ty: &Type) -> String {
    match ty {
        syn::Type::Path(p) => {
            let Some(segment) = p.path.segments.last() else {
                return rust_type_to_lua(ty).to_string();
            };

            if !matches!(segment.arguments, syn::PathArguments::None) {
                return rust_type_to_lua(ty).to_string();
            }

            let ident = segment.ident.to_string();
            let primitive = rust_type_to_lua(ty);
            if primitive != "table" || ident.ends_with("Id") {
                primitive.to_string()
            } else {
                ident
            }
        }
        _ => rust_type_to_lua(ty).to_string(),
    }
}

fn generate_to_lua_impl(fields: &Fields, name: &syn::Ident) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(named) => {
            let setters = named.named.iter().map(|field| {
                let field_ident = field.ident.as_ref().expect("named field");
                let field_name = field_ident.to_string();
                let field_ty = &field.ty;

                if is_vec2_type(field_ty) {
                    quote! {
                        table.set(
                            #field_name,
                            crate::scripting::lua_marshalling::write_named_vec2_table(lua, comp.#field_ident)?,
                        )?;
                    }
                } else if is_vec3_type(field_ty) {
                    quote! {
                        table.set(
                            #field_name,
                            crate::scripting::lua_marshalling::write_named_vec3_table(lua, comp.#field_ident)?,
                        )?;
                    }
                } else {
                    quote! {
                        table.set(#field_name, lua.to_value(&comp.#field_ident)?)?;
                    }
                }
            });

            quote! {
                use mlua::LuaSerdeExt;
                let comp = any
                    .downcast_ref::<#name>()
                    .expect(concat!("ComponentRegistry: type mismatch for ", stringify!(#name)));
                let table = lua.create_table()?;
                #(#setters)*
                Ok(mlua::Value::Table(table))
            }
        }
        _ => quote! {
            use mlua::LuaSerdeExt;
            let comp = any
                .downcast_ref::<#name>()
                .expect(concat!("ComponentRegistry: type mismatch for ", stringify!(#name)));
            lua.to_value(comp)
        },
    }
}

fn generate_from_lua_impl(fields: &Fields, name: &syn::Ident) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(named) => {
            let initializers = named.named.iter().map(|field| {
                let field_ident = field.ident.as_ref().expect("named field");
                let field_name = field_ident.to_string();
                let field_ty = &field.ty;

                if is_vec2_type(field_ty) {
                    quote! {
                        #field_ident: {
                            let value = table.get::<mlua::Table>(#field_name)?;
                            crate::scripting::lua_marshalling::read_named_vec2_table(&value, #field_name)?
                        }
                    }
                } else if is_vec3_type(field_ty) {
                    quote! {
                        #field_ident: {
                            let value = table.get::<mlua::Table>(#field_name)?;
                            crate::scripting::lua_marshalling::read_named_vec3_table(&value, #field_name)?
                        }
                    }
                } else {
                    quote! {
                        #field_ident: lua.from_value(table.get::<mlua::Value>(#field_name)?)?
                    }
                }
            });

            quote! {
                use mlua::LuaSerdeExt;
                let table = match value {
                    mlua::Value::Table(table) => table,
                    other => {
                        return Err(mlua::Error::FromLuaConversionError {
                            from: other.type_name(),
                            to: stringify!(#name).to_string(),
                            message: Some("expected table".into()),
                        });
                    }
                };
                let comp = #name {
                    #(#initializers),*
                };
                Ok(Box::new(comp) as Box<dyn std::any::Any>)
            }
        }
        _ => quote! {
            use mlua::LuaSerdeExt;
            let comp: #name = lua.from_value(value)?;
            Ok(Box::new(comp) as Box<dyn std::any::Any>)
        },
    }
}

fn is_vec2_type(ty: &Type) -> bool {
    matches!(ty, syn::Type::Path(p) if p.path.is_ident("Vec2"))
}

fn is_vec3_type(ty: &Type) -> bool {
    matches!(ty, syn::Type::Path(p) if p.path.is_ident("Vec3"))
}
