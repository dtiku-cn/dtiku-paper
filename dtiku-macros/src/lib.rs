mod cached;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn cached(args: TokenStream, input: TokenStream) -> TokenStream {
    cached::cached(args, input)
}