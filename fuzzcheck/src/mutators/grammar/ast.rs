extern crate self as fuzzcheck;

#[cfg(feature = "serde_json_serializer")]
use serde::{ser::SerializeTuple, Deserialize, Serialize};

/// An abstract syntax tree.
///
#[cfg_attr(
    feature = "serde_json_serializer",
    doc = "It can be serialized with [`SerdeSerializer`](crate::SerdeSerializer) on crate feature `serde_json_serializer`"
)]
#[cfg_attr(
    not(feature = "serde_json_serializer"),
    doc = "It can be serialized with `SerdeSerializer` on crate feature `serde_json_serializer`"
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AST {
    #[doc(hidden)]
    Token(char),
    #[doc(hidden)]
    Sequence(Vec<AST>),
    #[doc(hidden)]
    Box(Box<AST>),
}

// /// Like an abstract syntax tree, but augmented with the string indices that correspond to each node
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct ASTMap {
//     pub start_index: usize,
//     pub len: usize,
//     pub content: ASTMappingKind,
// }
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum ASTMappingKind {
//     Token,
//     Sequence(Vec<ASTMap>),
//     Box(Box<ASTMap>),
// }

impl AST {
    // #[no_coverage]
    // pub(crate) fn generate_string_and_ast_map_in(&self, s: &mut String, start_index: &mut usize) -> ASTMap {
    //     match self {
    //         AST::Token(c) => {
    //             let len = c.len_utf8();
    //             let orig_start_index = *start_index;
    //             s.push(*c);
    //             *start_index += len;
    //             ASTMap {
    //                 start_index: orig_start_index,
    //                 len,
    //                 content: ASTMappingKind::Token,
    //             }
    //         }
    //         AST::Sequence(asts) => {
    //             let original_start_idx = *start_index;
    //             let mut cs = vec![];
    //             for ast in asts {
    //                 let c = ast.generate_string_and_ast_map_in(s, start_index);
    //                 cs.push(c);
    //             }
    //             ASTMap {
    //                 start_index: original_start_idx,
    //                 len: *start_index - original_start_idx,
    //                 content: ASTMappingKind::Sequence(cs),
    //             }
    //         }
    //         AST::Box(ast) => {
    //             let mapping = ast.generate_string_and_ast_map_in(s, start_index);
    //             ASTMap {
    //                 start_index: mapping.start_index,
    //                 len: mapping.len,
    //                 content: ASTMappingKind::Box(Box::new(mapping)),
    //             }
    //         }
    //     }
    // }

    #[no_coverage]
    fn generate_string_in(&self, string: &mut String) {
        match self {
            AST::Token(c) => {
                string.push(*c);
            }
            AST::Sequence(asts) => {
                for ast in asts {
                    ast.generate_string_in(string);
                }
            }
            AST::Box(ast) => {
                ast.generate_string_in(string);
            }
        }
    }

    /// Converts the AST to its `String` representation
    #[allow(clippy::inherent_to_string)]
    #[no_coverage]
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(64);
        self.generate_string_in(&mut s);
        s
    }

    // #[no_coverage]
    // pub(crate) fn generate_string_and_ast_map(&self) -> (String, ASTMap) {
    //     let mut s = String::new();
    //     let mut start_index = 0;
    //     let c = self.generate_string_and_ast_map_in(&mut s, &mut start_index);
    //     (s, c)
    // }
    // #[no_coverage]
    // pub(crate) fn generate_string_and_ast_map_starting_at_idx(&self, idx: usize) -> (String, ASTMap) {
    //     let mut s = String::new();
    //     let mut start_index = idx;
    //     let c = self.generate_string_and_ast_map_in(&mut s, &mut start_index);
    //     (s, c)
    // }
}

// impl From<&AST> for ASTMap {
//     #[no_coverage]
//     fn from(ast: &AST) -> Self {
//         ast.generate_string_and_ast_map().1
//     }
// }
// impl From<AST> for String {
//     #[no_coverage]
//     fn from(ast: AST) -> Self {
//         ast.generate_string_and_ast_map().0
//     }
// }

/// A type that is exactly the same as AST so that I can derive most of the
/// Serialize/Deserialize implementation
#[cfg(feature = "serde_json_serializer")]
#[derive(Serialize, Deserialize)]
enum __AST {
    Token(char),
    Sequence(Vec<__AST>),
    Box(Box<__AST>),
}

#[cfg(feature = "serde_json_serializer")]
#[doc(cfg(feature = "serde_json_serializer"))]
impl Serialize for AST {
    #[no_coverage]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = self.to_string();
        let mut ser = serializer.serialize_tuple(2)?;
        ser.serialize_element(&string)?;
        let ast = unsafe { std::mem::transmute::<&AST, &__AST>(self) };
        ser.serialize_element(ast)?;
        ser.end()
    }
}
#[cfg(feature = "serde_json_serializer")]
#[doc(cfg(feature = "serde_json_serializer"))]
impl<'de> Deserialize<'de> for AST {
    #[no_coverage]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ast = <(String, __AST)>::deserialize(deserializer).map(|x| x.1)?;
        Ok(unsafe { std::mem::transmute::<__AST, AST>(ast) })
    }
}
