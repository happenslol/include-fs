use quote::quote;

#[proc_macro]
pub fn include_fs(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let input = syn::parse_macro_input!(input as syn::LitStr);

  let Ok(out_dir) = std::env::var("OUT_DIR") else {
    return quote! { compile_error!("OUT_DIR not set") }.into();
  };

  let mut input_value = input.value();

  let not_found_err = format!(
    "Bundle does not exist, did you add `include_fs::bundle(..., \"{}\")` to your build script?",
    input.value(),
  );

  input_value.push_str(".embed_fs");

  let bundle_path = std::path::Path::new(&input_value);
  if bundle_path.is_absolute() {
    return syn::Error::new_spanned(input, "Bundle path must be relative")
      .into_compile_error()
      .into();
  }

  let Ok(bundle_path) = std::path::Path::new(&out_dir)
    .join(input_value)
    .canonicalize()
  else {
    return syn::Error::new_spanned(input, not_found_err)
      .into_compile_error()
      .into();
  };

  if !bundle_path.starts_with(&out_dir) {
    return syn::Error::new_spanned(input, "Bundle path can not escape OUT_DIR")
      .into_compile_error()
      .into();
  }

  if !bundle_path.exists() {
    return syn::Error::new_spanned(input, not_found_err)
      .into_compile_error()
      .into();
  }

  let include_path = bundle_path
    .to_str()
    .expect("bundle path is not valid unicode");

  quote! {
    std::sync::LazyLock::new(|| {
      let archived_bytes: &[u8] = include_bytes!(#include_path);
      include_fs::IncludeFsInner::new(archived_bytes)
        .expect("Failed to initialize IncludeFs")
    })
  }
  .into()
}
