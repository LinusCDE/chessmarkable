/// TODO: Polish, rename properly and publish
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Lit, Meta, MetaNameValue};

/// This macro can be derives from.
/// It creates serde serialization from `to_string()` of the struct (Display trait expected)
/// and offers deserialization with `parse()` (FromStr trait expected)
///
/// Serde also wants a description of the expected data beeing deserialized
/// For that add the following to your struct:
/// `#[expected_data_description = "A string that continues: »I expect ...« without a dot the end"]`
#[proc_macro_derive(SerdeDisplayFromStr, attributes(expected_data_description))]
pub fn serde_display_fromstr(input: TokenStream) -> TokenStream {
    // Huge help: https://dev.to/jeikabu/rust-derive-macros-o38

    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    //input.attrs.app

    // Iterate over the struct's #[...] attributes
    let mut expected_data_description: Option<String> = None;
    for option in input.attrs.into_iter() {
        let option = option.parse_meta().unwrap();
        match option {
            // Match '#[ident = lit]' attributes. Match guard makes it '#[prefix = lit]'
            Meta::NameValue(MetaNameValue {
                ref path, ref lit, ..
            }) if path
                .get_ident()
                .expect("Failed to get ident of meta")
                .to_string()
                == "expected_data_description" =>
            {
                if let Lit::Str(lit) = lit {
                    expected_data_description = Some(lit.value());
                };
            }
            _ => {}
        }
    }
    let expected_data_description = expected_data_description.expect("Please add '#[expected_data_description = \"A string that continues: »I expect ...« without a dot the end\"]");

    // Build the output (gets appended after the derived struct/enum)
    let name = input.ident;

    let accompanying_visitor = format_ident!("{}Visitor", name);

    let serde_impls = quote! {
        // --------------------------------------------------
        // Appended by derive SerdeDisplayFromStr
        impl serde::ser::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                // Call to_string for the result (Display trait expected)
                serializer.serialize_str(&self.to_string())
            }
        }

        struct #accompanying_visitor { }

        impl<'de> serde::de::Visitor<'de> for #accompanying_visitor {
            // The type that our Visitor is going to produce.
            type Value = #name;

            // Format a message stating what data this Visitor expects to receive.
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(#expected_data_description)
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // Get struct/enum from String (FromStr trait expected)
                v.parse()
                    .map_err(|e| serde::de::Error::custom::<<#name as std::str::FromStr>::Err>(e))
            }
        }

        impl<'de> serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_any(#accompanying_visitor { })
            }
        }
        // --------------------------------------------------
    };
    serde_impls.into()
}
