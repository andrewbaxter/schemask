use {
    crate::{
        Maskoid,
        Schemask,
    },
    proc_macro2::TokenStream,
    quote::{
        format_ident,
        quote,
    },
    std::collections::HashMap,
};

struct CodeGen {
    extra_defs: Vec<TokenStream>,
}

impl CodeGen {
    fn new() -> Self {
        Self { extra_defs: vec![] }
    }

    /// Generates a top-level named type definition for a binding.
    /// Records and TaggedUnions become structs/enums; everything else becomes a type alias.
    fn gen_type_def(&mut self, name: &str, maskoid: &Maskoid) -> TokenStream {
        let ident = format_ident!("{}", name);
        match maskoid {
            Maskoid::Record(fields) => self.gen_record(name, fields),
            Maskoid::TaggedUnion(variants) => self.gen_tagged_union(name, variants),
            other => {
                let ty = self.gen_type_expr(other, name);
                quote! { pub type #ident = #ty; }
            },
        }
    }

    /// Returns a Rust type expression for the given maskoid.
    /// For Record and TaggedUnion, a helper type named after `hint` is added to extra_defs.
    /// For nested Option(Option(...)), a wrapper struct is added to extra_defs.
    fn gen_type_expr(&mut self, maskoid: &Maskoid, hint: &str) -> TokenStream {
        match maskoid {
            Maskoid::Null => quote! { () },
            Maskoid::String | Maskoid::ConstString(_) => quote! { String },
            Maskoid::Bool => quote! { bool },
            Maskoid::Int => quote! { i64 },
            Maskoid::Float => quote! { f64 },
            Maskoid::Ref(r) => {
                let ident = format_ident!("{}", r);
                quote! { #ident }
            },
            Maskoid::Option(inner) => {
                if matches!(inner.as_ref(), Maskoid::Option(_)) {
                    // Nested options: the inner option is encoded as {"element": <value>}.
                    // Generate a wrapper struct so Option<Wrapper> correctly serialises to
                    // either null (None) or {"element": ...} (Some(Wrapper { element })).
                    let wrapper_name = format!("{}Wrapper", hint);
                    let wrapper_ident = format_ident!("{}", wrapper_name);
                    let element_ty = self.gen_type_expr(inner, &format!("{}Inner", hint));
                    self.extra_defs.push(quote! {
                        #[derive(::serde::Serialize, ::serde::Deserialize)]
                        pub struct #wrapper_ident {
                            pub element: #element_ty,
                        }
                    });
                    quote! { Option<#wrapper_ident> }
                } else {
                    let inner_ty = self.gen_type_expr(inner, hint);
                    quote! { Option<#inner_ty> }
                }
            },
            Maskoid::Set(inner) | Maskoid::List(inner) => {
                let inner_ty = self.gen_type_expr(inner, hint);
                quote! { Vec<#inner_ty> }
            },
            Maskoid::StringMap(inner) => {
                let inner_ty = self.gen_type_expr(inner, hint);
                quote! { std::collections::HashMap<String, #inner_ty> }
            },
            Maskoid::Tuple(maskoids) => {
                let types: Vec<TokenStream> = maskoids
                    .iter()
                    .enumerate()
                    .map(|(i, m)| self.gen_type_expr(m, &format!("{}_{}", hint, i)))
                    .collect();
                // Trailing comma so single-element tuples are unambiguous.
                quote! { (#(#types,)*) }
            },
            Maskoid::Record(fields) => {
                let def = self.gen_record(hint, fields);
                self.extra_defs.push(def);
                let ident = format_ident!("{}", hint);
                quote! { #ident }
            },
            Maskoid::TaggedUnion(variants) => {
                let def = self.gen_tagged_union(hint, variants);
                self.extra_defs.push(def);
                let ident = format_ident!("{}", hint);
                quote! { #ident }
            },
        }
    }

    fn gen_record(&mut self, name: &str, fields: &HashMap<String, Maskoid>) -> TokenStream {
        let ident = format_ident!("{}", name);
        let mut sorted_fields: Vec<_> = fields.iter().collect();
        sorted_fields.sort_by_key(|(k, _)| k.as_str());
        let field_tokens: Vec<TokenStream> = sorted_fields
            .iter()
            .map(|(fname, fmaskoid)| {
                let fident = format_ident!("{}", fname);
                let hint = format!("{}{}", name, to_pascal_case(fname));
                match fmaskoid {
                    Maskoid::Option(_) => {
                        // Call gen_type_expr on the *full* Option maskoid rather than
                        // its inner, so Option(Option(...)) produces Option<Wrapper>
                        // rather than Option<Option<...>>.
                        let full_ty = self.gen_type_expr(fmaskoid, &hint);
                        quote! {
                            #[serde(skip_serializing_if = "Option::is_none", default)]
                            pub #fident: #full_ty
                        }
                    },
                    other => {
                        let ty = self.gen_type_expr(other, &hint);
                        quote! { pub #fident: #ty }
                    },
                }
            })
            .collect();
        quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            pub struct #ident {
                #(#field_tokens),*
            }
        }
    }

    fn gen_tagged_union(
        &mut self,
        name: &str,
        variants: &HashMap<String, Maskoid>,
    ) -> TokenStream {
        let ident = format_ident!("{}", name);
        let mut sorted_variants: Vec<_> = variants.iter().collect();
        sorted_variants.sort_by_key(|(k, _)| k.as_str());
        let variant_tokens: Vec<TokenStream> = sorted_variants
            .iter()
            .map(|(vname, vmaskoid)| {
                let vident = format_ident!("{}", vname);
                let hint = format!("{}{}", name, vname);
                let vty = self.gen_type_expr(vmaskoid, &hint);
                quote! { #vident(#vty) }
            })
            .collect();
        quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            pub enum #ident {
                #(#variant_tokens),*
            }
        }
    }
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize = true;
        } else if capitalize {
            result.extend(c.to_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}

pub fn generate_rust(schema: &Schemask) -> String {
    let mut codegen = CodeGen::new();
    let mut sorted_bindings: Vec<_> = schema.bindings.iter().collect();
    sorted_bindings.sort_by_key(|(k, _)| k.as_str());
    let binding_defs: Vec<TokenStream> = sorted_bindings
        .iter()
        .map(|(name, maskoid)| codegen.gen_type_def(name, maskoid))
        .collect();
    let extra_defs = codegen.extra_defs;
    let all: Vec<TokenStream> = extra_defs.into_iter().chain(binding_defs).collect();
    let tokens = quote! { #(#all)* };
    let code = tokens.to_string();
    genemichaels_lib::format_str(&code, &genemichaels_lib::FormatConfig::default())
        .map(|r| r.rendered)
        .unwrap_or(code)
}
