use quote::{quote, ToTokens};
use syn::{parse_macro_input, LitStr};
use url::{
    quirks::{internal_components, InternalComponents},
    Host, Url,
};

fn quote_option<T: ToTokens>(value: Option<T>) -> proc_macro2::TokenStream {
    if let Some(value) = value {
        quote! { ::core::option::Option::Some(#value) }
    } else {
        quote! { ::core::option::Option::None }
    }
}

/// Parse url at compile-time
///
/// ```
/// const URL: url::Url = url_macro::parse!("https://www.github.com");
/// # fn main() {
/// assert_eq!(URL, url::Url::parse("https://www.github.com").unwrap());
/// # }
/// ```
#[proc_macro]
pub fn parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as LitStr).value();

    let url = Url::parse(&input).unwrap();

    let serialization = url.as_str();
    let host = match url.host().unwrap_or(Host::Domain("")) {
        Host::Ipv4(address) => {
            let octets = &address.octets();
            quote! { ::url::Host::Ipv4(::std::net::Ipv4Addr::new(#(#octets),*)) }
        }
        Host::Ipv6(address) => {
            let octets = &address.octets();
            quote! { ::url::Host::Ipv6(::std::net::Ipv6Addr::new(#(#octets),*)) }
        }
        Host::Domain(domain) => quote! { ::url::Host::Domain(#domain) },
    };
    let InternalComponents {
        scheme_end,
        username_end,
        host_start,
        host_end,
        path_start,
        port,
        query_start,
        fragment_start,
    } = internal_components(&url);

    let port = quote_option(port);
    let query_start = quote_option(query_start);
    let fragment_start = quote_option(fragment_start);

    quote! {
        ::url::quirks::url_from_parts(
            #serialization,
            #host,
            ::url::quirks::InternalComponents {
                scheme_end: #scheme_end,
                username_end: #username_end,
                host_start: #host_start,
                host_end: #host_end,
                path_start: #path_start,
                port: #port,
                query_start: #query_start,
                fragment_start: #fragment_start,
            },
        )
    }
    .into()
}
