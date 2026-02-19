use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};
use proc_macro2::Span;
use syn::Ident;

#[proc_macro_derive(SerializeNumberStruct)]
pub fn serialise_number_struct(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let serialize_fields = match &ast.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => {
                let field_serializations = fields.named.iter().map(|field| {
                    let field_name = &field.ident;
                    let field_type = &field.ty;

                    let is_string = match field_type {
                        Type::Path(tp) => tp.path.get_ident().map_or(false, |i| i == "String"),
                        _ => false,
                    };

                    if is_string {
                        quote! {
                            let str_bytes = self.#field_name.as_bytes();
                            result.extend_from_slice(&(str_bytes.len() as u32).to_le_bytes());
                            result.extend_from_slice(str_bytes);
                        }
                    } else {
                        quote! {
                            result.extend_from_slice(&self.#field_name.to_le_bytes());
                        }
                    }
                });
                quote! { #(#field_serializations)* }
            }
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    quote! {
        impl Serialize for #name {
            fn serialize(&self) -> Vec<u8> {
                let mut result = Vec::new();
                #serialize_fields
                result
            }
        }
    }
    .into()
}

#[proc_macro_derive(DeserializeNumberStruct)]
pub fn deserialise_number_struct(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let (deserialize_fields, field_assignments) = match &ast.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => {
                let mut offset: usize = 0;
                let mut has_dynamic = false;
                let mut field_deserializations = Vec::new();
                let mut field_assignments = Vec::new();

                for field in &fields.named {
                    let field_name = &field.ident;
                    let field_type = &field.ty;

                    let type_str = match field_type {
                        Type::Path(tp) => tp
                            .path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| panic!("Unsupported type")),
                        _ => panic!("Unsupported type"),
                    };

                    match type_str.as_str() {
                        "String" => {
                            if !has_dynamic {
                                field_deserializations.push(quote! {
                                    let mut __cursor: usize = #offset;
                                });
                                has_dynamic = true;
                            }

                            field_deserializations.push(quote! {
                                let #field_name: String = {
                                    if base.len() < __cursor + 4 {
                                        return Err(Error);
                                    }
                                    let len_bytes: [u8; 4] = base[__cursor..__cursor + 4]
                                        .try_into()
                                        .map_err(|_| Error)?;
                                    let str_len = u32::from_le_bytes(len_bytes) as usize;
                                    __cursor += 4;
                                    if base.len() < __cursor + str_len {
                                        return Err(Error);
                                    }
                                    let s = String::from_utf8(
                                        base[__cursor..__cursor + str_len].to_vec()
                                    )
                                    .map_err(|_| Error)?;
                                    __cursor += str_len;
                                    s
                                };
                            });
                        }

                        _ => {
                            let (field_size, type_ident): (usize, Ident) = match type_str.as_str() {
                                "u8"  => (1, Ident::new("u8",  Span::call_site())),
                                "i8"  => (1, Ident::new("i8",  Span::call_site())),
                                "u16" => (2, Ident::new("u16", Span::call_site())),
                                "i16" => (2, Ident::new("i16", Span::call_site())),
                                "u32" => (4, Ident::new("u32", Span::call_site())),
                                "i32" => (4, Ident::new("i32", Span::call_site())),
                                "u64" => (8, Ident::new("u64", Span::call_site())),
                                "i64" => (8, Ident::new("i64", Span::call_site())),
                                other => panic!("Unsupported type: {}", other),
                            };
                            let size = field_size;

                            if has_dynamic {
                                field_deserializations.push(quote! {
                                    
                                    let #field_name: #type_ident = {
                                        if base.len() < __cursor + #size {
                                            return Err(Error);
                                        }
                                        let bytes: [u8; #size] = base[__cursor..__cursor + #size]
                                            .try_into()
                                            .map_err(|_| Error)?;
                                        __cursor += #size;
                                        #type_ident::from_le_bytes(bytes)
                                    };
                                });
                            } else {
                                let start_offset = offset;
                                let end_offset = offset + field_size;
                                field_deserializations.push(quote! {
                                    let #field_name: #type_ident = {
                                        let bytes: [u8; #size] = base[#start_offset..#end_offset]
                                            .try_into()
                                            .map_err(|_| Error)?;
                                        #type_ident::from_le_bytes(bytes)
                                    };
                                });
                                offset += field_size;
                            }
                        }
                    }

                    field_assignments.push(quote! { #field_name });
                }

                (field_deserializations, field_assignments)
            }
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    quote! {
        impl Deserialize for #name {
            fn deserialize(base: &[u8]) -> Result<Self, Error> {
                #(#deserialize_fields)*
                Ok(#name {
                    #(#field_assignments,)*
                })
            }
        }
    }
    .into()
}