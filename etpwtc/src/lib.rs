extern crate proc_macro;
use endec::{Endec, Secret};
use proc_macro2::{Group, Punct, Spacing, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, LitByteStr, LitInt, Result, Token,
};

struct Encrypted {
    key: LitByteStr,
    context: LitInt,
    plaintext: LitByteStr,
}

impl Parse for Encrypted {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: LitByteStr = input.parse()?;
        input.parse::<Token![:]>()?;
        let context: LitInt = input.parse()?;
        input.parse::<Token![,]>()?;
        let plaintext: LitByteStr = input.parse()?;
        Ok(Encrypted {
            key,
            context,
            plaintext,
        })
    }
}

struct ByteSlice<'a>(&'a [u8]);

impl ToTokens for ByteSlice<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Punct::new('&', Spacing::Joint));

        let mut elements = TokenStream::new();
        for b in self.0.iter() {
            b.to_tokens(&mut elements);
            elements.append(Punct::new(',', Spacing::Alone));
        }

        tokens.append(Group::new(proc_macro2::Delimiter::Bracket, elements));
    }
}

struct ByteArray<const N: usize>([u8; N]);

impl<const N: usize> ToTokens for ByteArray<N> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut elements = TokenStream::new();
        for b in self.0.iter() {
            b.to_tokens(&mut elements);
            elements.append(Punct::new(',', Spacing::Alone));
        }

        tokens.append(Group::new(proc_macro2::Delimiter::Bracket, elements));
    }
}

#[proc_macro]
pub fn encrypted(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Encrypted {
        key,
        context,
        plaintext,
    } = parse_macro_input!(input as Encrypted);

    let mut endec = Endec::new(context.base10_parse().unwrap());
    let Secret { nonce, ciphertext } = endec
        .enc(
            &Endec::make_key(key.value().as_slice()),
            plaintext.value().as_slice(),
        )
        .unwrap();

    let nonce = ByteArray(nonce);
    let ciphertext = ByteSlice(ciphertext);

    let output = quote! {
        endec::Secret {
            nonce: #nonce,
            ciphertext: #ciphertext
        }
    };

    output.into()
}
