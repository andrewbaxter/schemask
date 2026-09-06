use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{
        LitStr,
        parse_macro_input,
    },
};

#[proc_macro]
pub fn from_schemask(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let base_dir = match proc_macro::Span::call_site().local_file() {
        Some(source) => source.parent().map(|d| d.to_path_buf()).unwrap_or_default(),
        None => {
            return syn::Error::new(
                lit.span(),
                "from_schemask!: could not determine the source file path to resolve the schema path against",
            )
                .to_compile_error()
                .into();
        },
    };
    let joined = base_dir.join(&lit.value());
    let path = std::fs::canonicalize(&joined).unwrap_or(joined);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            return syn::Error::new(
                lit.span(),
                format!("from_schemask!: failed to read schema file {}: {}", path.display(), e),
            )
                .to_compile_error()
                .into();
        },
    };
    let schema = match serde_json::from_str::<schemask_core::Schemask>(&text) {
        Ok(s) => s,
        Err(e) => {
            return syn::Error::new(
                lit.span(),
                format!("from_schemask!: failed to parse schema file {}: {}", path.display(), e),
            )
                .to_compile_error()
                .into();
        },
    };
    let types = schemask_core::generate_rust_tokens(&schema);
    let path_str = path.to_string_lossy();
    return quote!{
        const _: &[u8] =::std::include_bytes!(#path_str);
        #types
    }.into();
}
