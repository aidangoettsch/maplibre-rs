use std::collections::HashMap;
use std::fmt::Formatter;
use std::fs::File;
use std::io::BufReader;
use serde::{Deserialize, Deserializer};
use serde::de::{MapAccess, Visitor};
use thiserror::Error;

#[derive(Deserialize, Debug)]
struct ExpressionSchema {
    interpolated: bool,
    parameters: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct EnumValueSchema {
    // doc: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum EnumValues {
    Numbers(Vec<usize>),
    Strings(Vec<String>),
    StringsWithSchema(HashMap<String, EnumValueSchema>),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ArrayType {
    Tuple(Vec<Box<ArrayType>>),
    SimpleReference(String),
    Reference(Box<JsonSchemaTypeReference>),
}

/// JSON of the form
/// ```
///     {
///         "type": "...",
///         ...
///     }
///
/// The type field indicates a reference to a type defined elsewhere or a primitive type. This
/// enum explicitly specifies the primitives,
#[derive(Debug)]
enum JsonSchemaTypeReference {
    String {
        required: bool,
        expression: Option<ExpressionSchema>,
        default: Option<String>,
    },
    Number {
        required: bool,
        expression: Option<ExpressionSchema>,
        default: Option<f64>,
    },
    Bool {
        required: bool,
        expression: Option<ExpressionSchema>,
        default: Option<bool>,
    },
    Array {
        required: bool,
        value: ArrayType,
        length: Option<usize>,
        expression: Option<ExpressionSchema>,
        defaulted: bool,
    },
    Enum {
        required: bool,
        values: EnumValues,
        expression: Option<ExpressionSchema>,
        default: Option<String>,
    },
    Reference {
        r#type: String,
        required: bool,
        expression: Option<ExpressionSchema>,
        defaulted: bool,
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DefaultValue {
    Number(f64),
    String(String),
    Bool(bool),
    Unknown(serde_json::Value),
}

impl<'de> Deserialize<'de> for JsonSchemaTypeReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        #[derive(Deserialize, Debug)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Type,
            Required,
            Value,
            Values,
            Length,
            Expression,
            Default,
            #[serde(other)]
            Unknown,
        }

        struct JsonSchemaTypeReferenceVisitor;

        impl<'de> Visitor<'de> for JsonSchemaTypeReferenceVisitor {
            type Value = JsonSchemaTypeReference;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("struct JsonSchemaTypeReference")
            }

            fn visit_map<V>(self, mut map: V) -> Result<JsonSchemaTypeReference, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut r#type = None;
                let mut required = None;
                let mut value = None;
                let mut values = None;
                let mut length = None;
                let mut expression = None;
                let mut default = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Type => {
                            if r#type.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type = Some(map.next_value::<String>()?);
                        }
                        Field::Required => {
                            if required.is_some() {
                                return Err(serde::de::Error::duplicate_field("required"));
                            }
                            required = Some(map.next_value::<bool>()?);
                        }
                        Field::Value => {
                            if value.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value = Some(map.next_value::<ArrayType>()?);
                        }
                        Field::Values => {
                            if values.is_some() {
                                return Err(serde::de::Error::duplicate_field("values"));
                            }
                            values = Some(map.next_value::<EnumValues>()?);
                        }
                        Field::Length => {
                            if length.is_some() {
                                return Err(serde::de::Error::duplicate_field("length"));
                            }
                            length = Some(map.next_value::<usize>()?);
                        }
                        Field::Expression => {
                            if expression.is_some() {
                                return Err(serde::de::Error::duplicate_field("expression"));
                            }
                            expression = Some(map.next_value::<ExpressionSchema>()?);
                        }
                        Field::Default => {
                            if default.is_some() {
                                return Err(serde::de::Error::duplicate_field("default"));
                            }
                            default = Some(map.next_value::<DefaultValue>()?);
                        }
                        Field::Unknown => {}
                    }
                };

                println!("visited field with type {:?}", r#type);

                let r#type = r#type.ok_or_else(|| serde::de::Error::missing_field("type"))?;
                let required = required.unwrap_or(false);

                match r#type.as_str() {
                    "string" => {
                        let default = match default {
                            Some(v) => {
                                match v {
                                    DefaultValue::String(s) => Some(s),
                                    _ => return Err(serde::de::Error::custom("Expected string default value for string type"))
                                }
                            },
                            None => None,
                        };

                        Ok(JsonSchemaTypeReference::String {
                            required,
                            expression,
                            default
                        })
                    },
                    "number" => {
                        let default = match default {
                            Some(v) => {
                                match v {
                                    DefaultValue::Number(s) => Some(s),
                                    _ => return Err(serde::de::Error::custom("Expected number default value for number type"))
                                }
                            },
                            None => None,
                        };

                        Ok(JsonSchemaTypeReference::Number {
                            required,
                            expression,
                            default
                        })
                    }
                    "boolean" => {
                        let default = match default {
                            Some(v) => {
                                match v {
                                    DefaultValue::Bool(s) => Some(s),
                                    _ => return Err(serde::de::Error::custom("Expected bool default value for bool type"))
                                }
                            },
                            None => None,
                        };

                        Ok(JsonSchemaTypeReference::Bool {
                            required,
                            expression,
                            default
                        })
                    },
                    "array" => {
                        Ok(JsonSchemaTypeReference::Array {
                            required,
                            value: value.ok_or_else(|| serde::de::Error::missing_field("value"))?,
                            length,
                            expression,
                            defaulted: default.is_some()
                        })
                    }
                    "enum" => {
                        let default = match default {
                            Some(v) => {
                                match v {
                                    DefaultValue::String(s) => Some(s),
                                    _ => return Err(serde::de::Error::custom("Expected string default value for enum type"))
                                }
                            },
                            None => None,
                        };

                        Ok(JsonSchemaTypeReference::Enum {
                            required,
                            expression,
                            default,
                            values: values.ok_or_else(|| serde::de::Error::missing_field("values"))?
                        })
                    },
                    _ => {
                        Ok(JsonSchemaTypeReference::Reference {
                            r#type,
                            required,
                            expression,
                            defaulted: default.is_some()
                        })
                    }
                }
            }
        }

        deserializer.deserialize_map(JsonSchemaTypeReferenceVisitor)
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum JsonSchemaTypedef {
    TypeReference(JsonSchemaTypeReference),
    UnionType(Vec<String>),
    Object(HashMap<String, JsonSchemaTypedef>),
}

#[derive(Deserialize, Debug)]
struct JsonSchema {
    #[serde(rename="$version")]
    version: u64,
    #[serde(rename="$root")]
    root: JsonSchemaTypedef,
    #[serde(flatten)]
    types: HashMap<String, JsonSchemaTypedef>,
}

#[derive(Error, Debug)]
pub enum StyleCodegenError {
    #[error("schema root was not an object")]
    SchemaRootNotObject,
    #[error("deserialization error")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
}

macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo::warning={}", format!($($tokens)*))
    }
}

fn generate_style_types() -> Result<(), StyleCodegenError> {
    let schema: JsonSchema = serde_json::from_reader(BufReader::new(File::open("./style-spec-v8.json")?))?;
    
    let JsonSchemaTypedef::Object(root) = schema.root else {
        return Err(StyleCodegenError::SchemaRootNotObject)
    };
    
    for (root_field_name, _) in root {
        p!("root field: {root_field_name}")
    }

    for (root_type_name, _) in schema.types {
        p!("root type: {root_type_name}")
    }

    Ok(())
}
fn main() {
    println!("cargo::rerun-if-changed=./style-spec-v8.json");
    println!("cargo::rerun-if-changed=./build.rs");

    generate_style_types().unwrap()
}