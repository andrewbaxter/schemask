use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{
        Attribute,
        Data,
        DeriveInput,
        Fields,
        Type,
        parse_macro_input,
    },
};

/// Derive `Schematize` for a struct or enum.
///
/// - Unit struct → `Maskoid::null()`
/// - Newtype struct (one unnamed field) → delegates to the inner type's `maskoid()`
/// - Named-field struct → `Maskoid::record(...)`
/// - Enum → `Maskoid::tagged_union(...)`
///
/// `///` doc comments on the type, fields, and enum variants are collected and stored
/// as the `description` on the produced maskoid.
#[proc_macro_derive(Schematize)]
pub fn derive_schematize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_doc = extract_doc(&input.attrs);

    let maskoid_expr = match &input.data {
        Data::Struct(s) => gen_struct_maskoid(&s.fields, type_doc.as_deref()),
        Data::Enum(e) => gen_enum_maskoid(e, type_doc.as_deref()),
        Data::Union(_) => {
            return syn::Error::new_spanned(&input.ident, "Schematize does not support unions")
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        impl #impl_generics ::schemask::Schematize for #name #ty_generics #where_clause {
            fn maskoid() -> ::schemask::Maskoid {
                #maskoid_expr
            }
        }
    };

    expanded.into()
}

fn gen_struct_maskoid(fields: &Fields, type_doc: Option<&str>) -> TokenStream2 {
    let base = match fields {
        Fields::Unit => quote! { ::schemask::Maskoid::null() },
        Fields::Unnamed(f) if f.unnamed.len() == 1 => {
            type_to_maskoid(&f.unnamed[0].ty)
        }
        Fields::Unnamed(f) => {
            let elems: Vec<_> = f.unnamed.iter().map(|f| {
                let m = type_to_maskoid(&f.ty);
                let fdoc = extract_doc(&f.attrs);
                apply_desc(quote! { ::schemask::MaskoidField::new(#m) }, fdoc.as_deref())
            }).collect();
            quote! {
                ::schemask::Maskoid::tuple(vec![ #(#elems),* ])
            }
        }
        Fields::Named(f) => {
            let entries: Vec<TokenStream2> = f.named.iter().map(|field| {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let field_doc = extract_doc(&field.attrs);
                let m = type_to_maskoid(&field.ty);
                let entry = apply_desc(quote! { ::schemask::MaskoidField::new(#m) }, field_doc.as_deref());
                quote! {
                    map.insert(#field_name.to_string(), #entry);
                }
            }).collect();
            quote! {
                {
                    let mut map = ::std::collections::HashMap::new();
                    #(#entries)*
                    ::schemask::Maskoid::record(map)
                }
            }
        }
    };
    apply_desc(base, type_doc)
}

fn gen_enum_maskoid(e: &syn::DataEnum, type_doc: Option<&str>) -> TokenStream2 {
    let entries: Vec<TokenStream2> = e.variants.iter().map(|variant| {
        let variant_name = variant.ident.to_string();
        let variant_doc = extract_doc(&variant.attrs);
        // Build the maskoid for the variant's payload (without variant description —
        // that goes into MaskoidField.description, not onto the maskoid itself).
        let maskoid = match &variant.fields {
            Fields::Unit => quote! { ::schemask::Maskoid::null() },
            Fields::Unnamed(f) if f.unnamed.len() == 1 => type_to_maskoid(&f.unnamed[0].ty),
            Fields::Unnamed(f) => {
                let elems: Vec<_> = f.unnamed.iter().map(|f| {
                    let m = type_to_maskoid(&f.ty);
                    let fdoc = extract_doc(&f.attrs);
                    apply_desc(quote! { ::schemask::MaskoidField::new(#m) }, fdoc.as_deref())
                }).collect();
                quote! { ::schemask::Maskoid::tuple(vec![ #(#elems),* ]) }
            }
            Fields::Named(f) => {
                let field_entries: Vec<TokenStream2> = f.named.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap().to_string();
                    let fdoc = extract_doc(&field.attrs);
                    let m = type_to_maskoid(&field.ty);
                    let entry = apply_desc(quote! { ::schemask::MaskoidField::new(#m) }, fdoc.as_deref());
                    quote! { map.insert(#name.to_string(), #entry); }
                }).collect();
                quote! {
                    {
                        let mut map = ::std::collections::HashMap::new();
                        #(#field_entries)*
                        ::schemask::Maskoid::record(map)
                    }
                }
            }
        };
        // Wrap maskoid in MaskoidField, attaching the variant doc comment.
        let variant_entry = apply_desc(
            quote! { ::schemask::MaskoidField::new(#maskoid) },
            variant_doc.as_deref(),
        );
        quote! {
            variants.insert(#variant_name.to_string(), #variant_entry);
        }
    }).collect();

    let base = quote! {
        {
            let mut variants = ::std::collections::HashMap::new();
            #(#entries)*
            ::schemask::Maskoid::tagged_union(variants)
        }
    };
    apply_desc(base, type_doc)
}

/// Convert a syn `Type` into a `Maskoid` expression.
fn type_to_maskoid(ty: &Type) -> TokenStream2 {
    quote! { <#ty as ::schemask::Schematize>::maskoid() }
}

/// Wrap `expr` in a `.with_description(...)` call if `desc` is Some.
fn apply_desc(expr: TokenStream2, desc: Option<&str>) -> TokenStream2 {
    match desc {
        None => expr,
        Some(d) => quote! { (#expr).with_description(#d) },
    }
}

/// Extract and join `///` doc comment lines from a set of attributes.
/// Returns `None` if there are no doc attributes.
fn extract_doc(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(s) = &expr_lit.lit {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect();

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}
