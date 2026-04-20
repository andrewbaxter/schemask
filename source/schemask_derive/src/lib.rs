use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    proc_macro_crate::{
        FoundCrate,
        crate_name,
    },
    quote::{
        format_ident,
        quote,
        quote_spanned,
    },
    syn::{
        Attribute,
        Data,
        DeriveInput,
        Fields,
        Type,
        parse_macro_input,
    },
};

#[proc_macro_derive(Maskoidy)]
pub fn derive_schematize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_doc = extract_doc(&input.attrs);
    let name_str = name.to_string();
    let span = name.span();
    let schemask_path = match crate_name("schemask") {
        Ok(FoundCrate::Itself) => quote!{
            crate
        },
        Ok(FoundCrate::Name(n)) => {
            let ident = format_ident!("{}", n);
            quote!{
                ::#ident
            }
        },
        Err(_) => quote!{
            ::schemask
        },
    };
    let inner_expr = match &input.data {
        Data::Struct(s) => gen_struct_maskoid(&s.fields, type_doc.as_deref(), &schemask_path),
        Data::Enum(e) => gen_enum_maskoid(e, type_doc.as_deref(), &schemask_path),
        Data::Union(_) => {
            return syn::Error::new_spanned(&input.ident, "Maskoidy does not support unions")
                .to_compile_error()
                .into();
        },
    };
    let schema_id_expr = quote_spanned!{
        span => :: std:: concat !(::std::file!(), ":", ::std::line!())
    };
    let expanded = quote!{
        impl #impl_generics #schemask_path:: Maskoidy for #name #ty_generics #where_clause {
            fn schema_id() -> &'static str {
                #schema_id_expr
            }
            fn schema_name() -> &'static str {
                #name_str
            }
            fn maskoid(
                seen: & mut:: std:: collections:: HashSet <&'static str >,
                bindings: & mut:: std:: collections:: HashMap <:: std:: string:: String,
                #schemask_path:: Maskoid >,
            ) -> #schemask_path:: Maskoid {
                let __id = < Self as #schemask_path:: Maskoidy >:: schema_id();
                let __name = < Self as #schemask_path:: Maskoidy >:: schema_name();
                if seen.contains(__id) {
                    return #schemask_path:: Maskoid:: ref_(__name);
                }
                seen.insert(__id);
                let __result = {
                    #inner_expr
                };
                bindings.insert(__name.to_string(), __result);
                return #schemask_path:: Maskoid:: ref_(__name);
            }
        }
    };
    return expanded.into();
}

fn gen_struct_maskoid(fields: &Fields, type_doc: Option<&str>, schemask_path: &TokenStream2) -> TokenStream2 {
    let base = match fields {
        Fields::Unit => quote!{
            #schemask_path:: Maskoid:: null()
        },
        Fields::Unnamed(f) if f.unnamed.len() == 1 => {
            type_to_maskoid(&f.unnamed[0].ty, schemask_path)
        },
        Fields::Unnamed(f) => {
            let elems: Vec<_> = f.unnamed.iter().map(|f| {
                let m = type_to_maskoid(&f.ty, schemask_path);
                let fdoc = extract_doc(&f.attrs);
                apply_desc(quote!{
                    #schemask_path:: MaskoidField:: new(#m)
                }, fdoc.as_deref())
            }).collect();
            quote!{
                #schemask_path:: Maskoid:: tuple(vec![#(#elems), *])
            }
        },
        Fields::Named(f) => {
            let entries: Vec<TokenStream2> = f.named.iter().map(|field| {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let field_doc = extract_doc(&field.attrs);
                let m = type_to_maskoid(&field.ty, schemask_path);
                let entry = apply_desc(quote!{
                    #schemask_path:: MaskoidField:: new(#m)
                }, field_doc.as_deref());
                quote!{
                    map.insert(#field_name.to_string(), #entry);
                }
            }).collect();
            quote!{
                {
                    let mut map = ::std::collections::HashMap::new();
                    #(#entries) * #schemask_path:: Maskoid:: record(map)
                }
            }
        },
    };
    return apply_desc(base, type_doc);
}

fn gen_enum_maskoid(e: &syn::DataEnum, type_doc: Option<&str>, schemask_path: &TokenStream2) -> TokenStream2 {
    let entries: Vec<TokenStream2> = e.variants.iter().map(|variant| {
        let variant_name = variant.ident.to_string();
        let variant_doc = extract_doc(&variant.attrs);
        let maskoid = match &variant.fields {
            Fields::Unit => quote!{
                #schemask_path:: Maskoid:: null()
            },
            Fields::Unnamed(f) if f.unnamed.len() == 1 => type_to_maskoid(&f.unnamed[0].ty, schemask_path),
            Fields::Unnamed(f) => {
                let elems: Vec<_> = f.unnamed.iter().map(|f| {
                    let m = type_to_maskoid(&f.ty, schemask_path);
                    let fdoc = extract_doc(&f.attrs);
                    apply_desc(quote!{
                        #schemask_path:: MaskoidField:: new(#m)
                    }, fdoc.as_deref())
                }).collect();
                quote!{
                    #schemask_path:: Maskoid:: tuple(vec![#(#elems), *])
                }
            },
            Fields::Named(f) => {
                let field_entries: Vec<TokenStream2> = f.named.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap().to_string();
                    let fdoc = extract_doc(&field.attrs);
                    let m = type_to_maskoid(&field.ty, schemask_path);
                    let entry = apply_desc(quote!{
                        #schemask_path:: MaskoidField:: new(#m)
                    }, fdoc.as_deref());
                    quote!{
                        map.insert(#name.to_string(), #entry);
                    }
                }).collect();
                quote!{
                    {
                        let mut map = ::std::collections::HashMap::new();
                        #(#field_entries) * #schemask_path:: Maskoid:: record(map)
                    }
                }
            },
        };
        let variant_entry = apply_desc(quote!{
            #schemask_path:: MaskoidField:: new(#maskoid)
        }, variant_doc.as_deref());
        quote!{
            variants.insert(#variant_name.to_string(), #variant_entry);
        }
    }).collect();
    let base = quote!{
        {
            let mut variants = ::std::collections::HashMap::new();
            #(#entries) * #schemask_path:: Maskoid:: tagged_union(variants)
        }
    };
    return apply_desc(base, type_doc);
}

fn type_to_maskoid(ty: &Type, schemask_path: &TokenStream2) -> TokenStream2 {
    return quote!{
        <#ty as #schemask_path:: Maskoidy >:: maskoid(seen, bindings)
    };
}

fn apply_desc(expr: TokenStream2, desc: Option<&str>) -> TokenStream2 {
    return match desc {
        None => expr,
        Some(d) => quote!{
            (#expr).with_description(#d)
        },
    };
}

fn extract_doc(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs.iter().filter_map(|attr| {
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
        return None;
    }).collect();
    if lines.is_empty() {
        return None;
    } else {
        return Some(lines.join("\n"));
    }
}
