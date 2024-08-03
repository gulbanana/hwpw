extern crate proc_macro;
use etpwtc_runtime::{Endec, Secret};
use proc_macro2::{Group, Punct, Spacing, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, LitByteStr, Result, Token,
};

struct Encrypted {
    key: LitByteStr,
    plaintext: LitByteStr,
}

impl Parse for Encrypted {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: LitByteStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let plaintext: LitByteStr = input.parse()?;
        Ok(Encrypted { key, plaintext })
    }
}

struct ByteArray<'a>(&'a [u8]);

impl ToTokens for ByteArray<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut elements = TokenStream::new();
        for b in self.0.iter() {
            b.to_tokens(&mut elements);
            elements.append(Punct::new(',', Spacing::Alone));
        }

        tokens.append(Group::new(proc_macro2::Delimiter::Bracket, elements));
    }
}

static mut CONTEXT: u8 = 0;

#[proc_macro]
pub fn encrypted(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Encrypted { key, plaintext } = parse_macro_input!(input as Encrypted);

    let mut endec = Endec::new(unsafe { CONTEXT });
    unsafe {
        CONTEXT += 1;
    }

    let Secret {
        nonce,
        len,
        ciphertext,
    } = endec
        .enc::<64>(
            &Endec::make_key(key.value().as_slice()),
            plaintext.value().as_slice(),
        )
        .unwrap();

    let nonce = ByteArray(&nonce);
    let ciphertext = ByteArray(ciphertext.as_slice());

    let output = quote! {
        Secret {
            nonce: #nonce,
            len: #len,
            ciphertext: #ciphertext
        }
    };

    output.into()
}
