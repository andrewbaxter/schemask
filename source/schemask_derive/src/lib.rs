use {
    proc_macro::TokenStream,
    proc_macro2::{
        TokenStream as TokenStream2,
        TokenTree,
    },
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

fn apply_desc(expr: TokenStream2, desc: Option<&str>) -> TokenStream2 {
    return match desc {
        None => expr,
        Some(d) => quote!{
            (#expr).with_description(#d)
        },
    };
}

#[proc_macro_derive(Maskoidy)]
pub fn derive_schematize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    if let Some(err) = (|| -> Option<TokenStream2> {
        for key in ["tag", "content", "untagged"] {
            if serde_has(&input.attrs, key) {
                return Some(
                    syn::Error::new_spanned(
                        &input.ident,
                        format!(
                            "Maskoidy does not support #[serde({})]: schemask unions are always externally tagged",
                            key
                        ),
                    ).to_compile_error(),
                );
            }
        }
        if let Data::Enum(e) = &input.data {
            for variant in &e.variants {
                if serde_has(&variant.attrs, "untagged") {
                    return Some(
                        syn::Error::new_spanned(
                            &variant.ident,
                            "Maskoidy does not support #[serde(untagged)]: schemask unions are always externally tagged",
                        ).to_compile_error(),
                    );
                }
            }
        }
        return None;
    })() {
        return err.into();
    }
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_doc = extract_doc(&input.attrs);
    let name_str = name.to_string();
    let span = name.span();
    let schemask_path = (|| -> TokenStream2 {
        for name in ["schemask", "schemask_core"] {
            match crate_name(name) {
                Ok(FoundCrate::Itself) => return quote!{
                    crate
                },
                Ok(FoundCrate::Name(n)) => {
                    let ident = format_ident!("{}", n);
                    return quote!{
                        ::#ident
                    };
                },
                Err(_) => continue,
            }
        }
        return quote!{
            ::schemask
        };
    })();
    let inner_expr = match &input.data {
        Data::Struct(s) => (|| -> TokenStream2 {
            let fields = &s.fields;
            let schemask_path = &schemask_path;
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
                        apply_desc(quote!{
                            #schemask_path:: MaskoidField:: new(#m)
                        }, extract_doc(&f.attrs).as_deref())
                    }).collect();
                    quote!{
                        #schemask_path:: Maskoid:: tuple(vec![#(#elems), *])
                    }
                },
                Fields::Named(f) => {
                    let entries: Vec<TokenStream2> = f.named.iter().map(|field| {
                        let field_name = field_key(field, rename_rule(&input.attrs, "rename_all"));
                        let m = type_to_maskoid(&field.ty, schemask_path);
                        let entry = apply_desc(quote!{
                            #schemask_path:: MaskoidField:: new(#m)
                        }, extract_doc(&field.attrs).as_deref());
                        quote!{
                            map.insert(#field_name.to_string(), #entry);
                        }
                    }).collect();
                    quote!{
                        {
                            let mut map = ::std::collections::BTreeMap::new();
                            #(#entries) * #schemask_path:: Maskoid:: record(map)
                        }
                    }
                },
            };
            return apply_desc(base, type_doc.as_deref());
        })(),
        Data::Enum(e) => (|| -> TokenStream2 {
            let schemask_path = &schemask_path;
            let entries: Vec<TokenStream2> = e.variants.iter().map(|variant| {
                let variant_name =
                    serde_value(
                        &variant.attrs,
                        "rename",
                    ).unwrap_or_else(
                        || rename_rule(&input.attrs, "rename_all").apply_to_variant(&unraw(&variant.ident)),
                    );
                let field_rule = match rename_rule(&variant.attrs, "rename_all") {
                    RenameRule::None => rename_rule(&input.attrs, "rename_all_fields"),
                    r => r,
                };
                let maskoid = match &variant.fields {
                    Fields::Unit => quote!{
                        #schemask_path:: Maskoid:: null()
                    },
                    Fields::Unnamed(f) if f.unnamed.len() == 1 => type_to_maskoid(&f.unnamed[0].ty, schemask_path),
                    Fields::Unnamed(f) => {
                        let elems: Vec<_> = f.unnamed.iter().map(|f| {
                            let m = type_to_maskoid(&f.ty, schemask_path);
                            apply_desc(quote!{
                                #schemask_path:: MaskoidField:: new(#m)
                            }, extract_doc(&f.attrs).as_deref())
                        }).collect();
                        quote!{
                            #schemask_path:: Maskoid:: tuple(vec![#(#elems), *])
                        }
                    },
                    Fields::Named(f) => {
                        let field_entries: Vec<TokenStream2> = f.named.iter().map(|field| {
                            let name = field_key(field, field_rule);
                            let m = type_to_maskoid(&field.ty, schemask_path);
                            let entry = apply_desc(quote!{
                                #schemask_path:: MaskoidField:: new(#m)
                            }, extract_doc(&field.attrs).as_deref());
                            quote!{
                                map.insert(#name.to_string(), #entry);
                            }
                        }).collect();
                        quote!{
                            {
                                let mut map = ::std::collections::BTreeMap::new();
                                #(#field_entries) * #schemask_path:: Maskoid:: record(map)
                            }
                        }
                    },
                };
                let variant_entry = apply_desc(quote!{
                    #schemask_path:: MaskoidField:: new(#maskoid)
                }, extract_doc(&variant.attrs).as_deref());
                quote!{
                    variants.insert(#variant_name.to_string(), #variant_entry);
                }
            }).collect();
            return apply_desc(quote!{
                {
                    let mut variants = ::std::collections::BTreeMap::new();
                    #(#entries) * #schemask_path:: Maskoid:: tagged_union(variants)
                }
            }, type_doc.as_deref());
        })(),
        Data::Union(_) => {
            return syn::Error::new_spanned(&input.ident, "Maskoidy does not support unions")
                .to_compile_error()
                .into();
        },
    };
    let schema_id_expr = quote_spanned!{
        span => :: std:: concat !(::std::file!(), ":", ::std::line!())
    };
    return quote!{
        impl #impl_generics #schemask_path:: Maskoidy for #name #ty_generics #where_clause {
            fn schema_id() -> &'static str {
                #schema_id_expr
            }
            fn schema_name() -> &'static str {
                #name_str
            }
            fn maskoid(
                seen: & mut:: std:: collections:: HashSet <&'static str >,
                bindings: & mut:: std:: collections:: BTreeMap <:: std:: string:: String,
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
    }.into();
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

fn field_key(field: &syn::Field, rule: RenameRule) -> String {
    return serde_value(
        &field.attrs,
        "rename",
    ).unwrap_or_else(|| rule.apply_to_field(&unraw(field.ident.as_ref().unwrap())));
}

fn lit_str_value(tt: &TokenTree) -> Option<String> {
    let TokenTree::Literal(l) = tt else {
        return None;
    };
    let s = l.to_string();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return Some(s[1 .. s.len() - 1].to_string());
    }
    return None;
}

fn rename_rule(attrs: &[Attribute], key: &str) -> RenameRule {
    return serde_value(attrs, key).and_then(|s| Some(match s.as_str() {
        "lowercase" => RenameRule::Lower,
        "UPPERCASE" => RenameRule::Upper,
        "PascalCase" => RenameRule::Pascal,
        "camelCase" => RenameRule::Camel,
        "snake_case" => RenameRule::Snake,
        "SCREAMING_SNAKE_CASE" => RenameRule::ScreamingSnake,
        "kebab-case" => RenameRule::Kebab,
        "SCREAMING-KEBAB-CASE" => RenameRule::ScreamingKebab,
        _ => return None,
    })).unwrap_or(RenameRule::None);
}

#[derive(Clone, Copy, PartialEq)]
enum RenameRule {
    Camel,
    Kebab,
    Lower,
    None,
    Pascal,
    ScreamingKebab,
    ScreamingSnake,
    Snake,
    Upper,
}

impl RenameRule {
    fn apply_to_field(&self, field: &str) -> String {
        return match self {
            RenameRule::None | RenameRule::Lower | RenameRule::Snake => field.to_string(),
            RenameRule::Upper | RenameRule::ScreamingSnake => field.to_ascii_uppercase(),
            RenameRule::Pascal => {
                let mut out = String::new();
                let mut capitalize = true;
                for ch in field.chars() {
                    if ch == '_' {
                        capitalize = true;
                    } else if capitalize {
                        out.push(ch.to_ascii_uppercase());
                        capitalize = false;
                    } else {
                        out.push(ch);
                    }
                }
                out
            },
            RenameRule::Camel => {
                let pascal = RenameRule::Pascal.apply_to_field(field);
                pascal[..1].to_ascii_lowercase() + &pascal[1..]
            },
            RenameRule::Kebab => field.replace('_', "-"),
            RenameRule::ScreamingKebab => field.to_ascii_uppercase().replace('_', "-"),
        };
    }

    fn apply_to_variant(&self, variant: &str) -> String {
        return match self {
            RenameRule::None | RenameRule::Pascal => variant.to_string(),
            RenameRule::Lower => variant.to_ascii_lowercase(),
            RenameRule::Upper => variant.to_ascii_uppercase(),
            RenameRule::Camel => variant[..1].to_ascii_lowercase() + &variant[1..],
            RenameRule::Snake => {
                let mut out = String::new();
                for (i, ch) in variant.char_indices() {
                    if i > 0 && ch.is_uppercase() {
                        out.push('_');
                    }
                    out.extend(ch.to_lowercase());
                }
                out
            },
            RenameRule::ScreamingSnake => RenameRule::Snake.apply_to_variant(variant).to_ascii_uppercase(),
            RenameRule::Kebab => RenameRule::Snake.apply_to_variant(variant).replace('_', "-"),
            RenameRule::ScreamingKebab => RenameRule::ScreamingSnake.apply_to_variant(variant).replace('_', "-"),
        };
    }
}

fn serde_has(attrs: &[Attribute], key: &str) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            continue;
        };
        for tok in list.tokens.clone() {
            if let TokenTree::Ident(ident) = tok {
                if ident == key {
                    return true;
                }
            }
        }
    }
    return false;
}

fn serde_value(attrs: &[Attribute], key: &str) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            continue;
        };
        let toks: Vec<TokenTree> = list.tokens.clone().into_iter().collect();
        for (i, tok) in toks.iter().enumerate() {
            let TokenTree::Ident(ident) = tok else {
                continue;
            };
            if ident != key {
                continue;
            }
            if let (Some(TokenTree::Punct(p)), Some(v)) = (toks.get(i + 1), toks.get(i + 2)) {
                if p.as_char() == '=' {
                    if let Some(s) = lit_str_value(v) {
                        return Some(s);
                    }
                }
            }
            if let Some(TokenTree::Group(g)) = toks.get(i + 1) {
                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                for want in ["serialize", "deserialize"] {
                    for (j, t) in inner.iter().enumerate() {
                        let TokenTree::Ident(id) = t else {
                            continue;
                        };
                        if id != want {
                            continue;
                        }
                        if let (Some(TokenTree::Punct(p)), Some(v)) = (inner.get(j + 1), inner.get(j + 2)) {
                            if p.as_char() == '=' {
                                if let Some(s) = lit_str_value(v) {
                                    return Some(s);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return None;
}

fn type_to_maskoid(ty: &Type, schemask_path: &TokenStream2) -> TokenStream2 {
    return quote!{
        <#ty as #schemask_path:: Maskoidy >:: maskoid(seen, bindings)
    };
}

fn unraw(ident: &syn::Ident) -> String {
    let s = ident.to_string();
    return s.strip_prefix("r#").unwrap_or(&s).to_string();
}
