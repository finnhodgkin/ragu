use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alpha1, alphanumeric1, multispace1},
    combinator::{opt, recognize},
    multi::{many0, separated_list1},
    sequence::pair,
    IResult,
};

/// Represents a parsed module declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDeclaration {
    pub name: String,
}

/// Represents a parsed import statement
#[derive(Debug, Clone, PartialEq)]
pub struct ImportStatement {
    pub module_name: String,
}

/// Parse result containing module declaration and imports
#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub module: Option<ModuleDeclaration>,
    pub imports: Vec<ImportStatement>,
}

/// Parse a complete PureScript file
pub fn parse_purescript_file(input: &str) -> IResult<&str, ParseResult> {
    let (input, _) = skip_comments_and_whitespace(input)?;
    let (input, module) = opt(parse_module_declaration)(input)?;
    let (input, _) = skip_comments_and_whitespace(input)?;
    let (input, imports) = many0(parse_import_statement)(input)?;

    Ok((input, ParseResult { module, imports }))
}

/// Parse a module declaration
fn parse_module_declaration(input: &str) -> IResult<&str, ModuleDeclaration> {
    let (input, _) = tag("module")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = parse_module_name(input)?;
    let (input, _) = skip_comments_and_whitespace(input)?;
    // Skip any exports list - we don't need to parse it
    let (input, _) = opt(parse_exports_list)(input)?;
    let (input, _) = skip_comments_and_whitespace(input)?;
    let (input, _) = tag("where")(input)?;

    Ok((input, ModuleDeclaration { name }))
}

/// Parse an import statement
fn parse_import_statement(input: &str) -> IResult<&str, ImportStatement> {
    // Skip any leading whitespace/comments before the import statement
    let (input, _) = skip_comments_and_whitespace(input)?;

    let (input, _) = tag("import")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, module_name) = parse_module_name(input)?;
    let (input, _) = skip_comments_and_whitespace(input)?;

    // Skip any exports list - we don't need to parse it
    let (input, _) = opt(parse_exports_list)(input)?;
    let (input, _) = skip_comments_and_whitespace(input)?;

    // Skip any hiding clause - we don't need to parse it
    let (input, _) = opt(parse_hiding_clause)(input)?;
    let (input, _) = skip_comments_and_whitespace(input)?;

    // Skip any alias - we don't need to parse it
    let (input, _) = opt(parse_alias)(input)?;

    Ok((input, ImportStatement { module_name }))
}

/// Parse a module name (e.g., "Data.Maybe")
fn parse_module_name(input: &str) -> IResult<&str, String> {
    let (input, name) = recognize(separated_list1(
        tag("."),
        pair(alpha1, many0(alphanumeric1)),
    ))(input)?;
    Ok((input, name.to_string()))
}

/// Parse an alias clause (e.g., "as Maybe" or "as Client.GqlError")
fn parse_alias(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("as")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, alias) = parse_module_alias(input)?;
    Ok((input, alias))
}

/// Parse a module alias that can contain dots (e.g., "Client.GqlError")
fn parse_module_alias(input: &str) -> IResult<&str, String> {
    let (input, alias) = recognize(separated_list1(tag("."), parse_identifier))(input)?;
    Ok((input, alias.to_string()))
}

/// Parse a hiding clause (e.g., "hiding (fromMaybe)")
fn parse_hiding_clause(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("hiding")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = parse_exports_list(input)?;
    Ok((input, ()))
}

/// Parse an exports list (e.g., "(Maybe(..), maybe)")
fn parse_exports_list(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("(")(input)?;
    let (input, _) = skip_comments_and_whitespace(input)?;
    // Skip the contents of the exports list - we don't need to parse it
    let (input, _) = take_until_closing_paren(input)?;
    let (input, _) = skip_comments_and_whitespace(input)?;
    let (input, _) = tag(")")(input)?;
    Ok((input, ()))
}

/// Take everything until we find a closing parenthesis, handling nested parentheses
fn take_until_closing_paren(input: &str) -> IResult<&str, &str> {
    let mut depth = 0;
    let mut chars = input.char_indices();

    while let Some((i, ch)) = chars.next() {
        match ch {
            '(' => {
                depth += 1;
            }
            ')' => {
                if depth == 0 {
                    return Ok((&input[i..], &input[..i]));
                }
                depth -= 1;
            }
            _ => {
                // Continue to next character
            }
        }
    }

    // If we get here, we didn't find a closing parenthesis
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

/// Parse an identifier
fn parse_identifier(input: &str) -> IResult<&str, String> {
    let (input, ident) = recognize(pair(
        alpha1,
        many0(alt((alphanumeric1, tag("_"), tag("'")))),
    ))(input)?;
    Ok((input, ident.to_string()))
}

/// Skip comments and whitespace
fn skip_comments_and_whitespace(input: &str) -> IResult<&str, ()> {
    let (input, _) = many0(alt((multispace1, parse_line_comment, parse_block_comment)))(input)?;
    Ok((input, ()))
}

/// Parse a line comment (-- comment)
fn parse_line_comment(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("--")(input)?;
    let (input, comment) = take_while(|c| c != '\n')(input)?;
    Ok((input, comment))
}

/// Parse a block comment ({- comment -})
fn parse_block_comment(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("{-")(input)?;
    // Find the closing -} by looking for it in the remaining input
    let mut chars = input.char_indices();
    let mut depth = 1; // We're already inside one block comment
    let mut comment_end = 0;

    while let Some((i, ch)) = chars.next() {
        if ch == '{' && input[i..].starts_with("{-") {
            depth += 1;
        } else if ch == '-' && input[i..].starts_with("-}") {
            depth -= 1;
            if depth == 0 {
                comment_end = i;
                break;
            }
        }
    }

    if depth == 0 {
        let comment = &input[..comment_end];
        let remaining = &input[comment_end + 2..]; // Skip the -}
        Ok((remaining, comment))
    } else {
        // If we get here, we didn't find a closing -}
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test cases for simple imports
    #[test]
    fn test_simple_import() {
        let input = "import Data.Maybe";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_import_with_exports() {
        let input = "import Data.Maybe (Maybe(..), maybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_import_with_alias() {
        let input = "import Data.Maybe as Maybe";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_import_with_hiding() {
        let input = "import Data.Maybe hiding (fromMaybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_import_with_exports_and_alias() {
        let input = "import Data.Maybe (Maybe(..), maybe) as Maybe";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_import_with_exports_and_hiding() {
        let input = "import Data.Maybe (Maybe(..), maybe) hiding (fromMaybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_import_with_exports_alias_and_hiding() {
        let input = "import Data.Maybe (Maybe(..), maybe) as Maybe hiding (fromMaybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    // Test cases for multiline imports
    #[test]
    fn test_multiline_simple_import() {
        let input = "import\nData.Maybe";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_import_with_exports() {
        let input = "import\nData.Maybe (Maybe(..), maybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_import_with_alias() {
        let input = "import\nData.Maybe as Maybe";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_import_with_hiding() {
        let input = "import\nData.Maybe hiding (fromMaybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_import_with_exports_and_alias() {
        let input = "import\nData.Maybe (Maybe(..), maybe) as Maybe";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_import_with_exports_and_hiding() {
        let input = "import\nData.Maybe (Maybe(..), maybe) hiding (fromMaybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_import_with_exports_alias_and_hiding() {
        let input = "import\nData.Maybe (Maybe(..), maybe) as Maybe hiding (fromMaybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_exports_and_alias_and_hiding() {
        let input = "import\nData.Maybe (\nMaybe(..), \nmaybe\n) as Maybe hiding (fromMaybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    // Test cases for module declarations
    #[test]
    fn test_simple_module() {
        let input = "module Data.Maybe where";
        let result = parse_module_declaration(input);
        assert!(result.is_ok());
        let (_, module) = result.unwrap();
        assert_eq!(module.name, "Data.Maybe");
    }

    #[test]
    fn test_module_with_exports() {
        let input = "module Data.Maybe (Maybe(..), maybe) where";
        let result = parse_module_declaration(input);
        assert!(result.is_ok());
        let (_, module) = result.unwrap();
        assert_eq!(module.name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_module() {
        let input = "module\nData.Maybe\nwhere";
        let result = parse_module_declaration(input);
        assert!(result.is_ok());
        let (_, module) = result.unwrap();
        assert_eq!(module.name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_module_with_exports() {
        let input = "module\nData.Maybe\n(Maybe(..), maybe)\nwhere";
        let result = parse_module_declaration(input);
        assert!(result.is_ok());
        let (_, module) = result.unwrap();
        assert_eq!(module.name, "Data.Maybe");
    }

    #[test]
    fn test_multiline_module_with_exports_and_alias() {
        let input = "module\nData.Maybe\n(Maybe(..), maybe)\nas Maybe\nwhere";
        let result = parse_module_declaration(input);
        // This should fail because module declarations don't support aliases
        assert!(result.is_err());
    }

    #[test]
    fn test_multiline_module_with_exports_and_hiding() {
        let input = "module\nData.Maybe\n(Maybe(..), maybe)\nhiding (fromMaybe)\nwhere";
        let result = parse_module_declaration(input);
        // This should fail because module declarations don't support hiding
        assert!(result.is_err());
    }

    #[test]
    fn test_multiline_module_with_exports_alias_and_hiding() {
        let input = "module\nData.Maybe\n(Maybe(..), maybe)\nas Maybe\nhiding (fromMaybe)\nwhere";
        let result = parse_module_declaration(input);
        // This should fail because module declarations don't support aliases or hiding
        assert!(result.is_err());
    }

    // Test cases for comments
    #[test]
    fn test_import_with_line_comment() {
        let input = "import Data.Maybe -- comment";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_import_with_block_comment() {
        let input = "import Data.Maybe {- comment -}";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    // Test cases for complex module names
    #[test]
    fn test_complex_module_name() {
        let input = "import Data.Maybe.Util";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe.Util");
    }

    #[test]
    fn test_module_name_with_numbers() {
        let input = "import Data.Maybe2";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe2");
    }

    // Test cases for edge cases
    #[test]
    fn test_empty_exports_list() {
        let input = "import Data.Maybe ()";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_export_with_underscore() {
        let input = "import Data.Maybe (from_maybe)";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_export_with_prime() {
        let input = "import Data.Maybe (maybe')";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    // Test cases for complete file parsing
    #[test]
    fn test_complete_file_parsing() {
        let input = r#"module Test where

import Prelude
import Data.Maybe (Maybe(..), maybe) as Maybe
import Data.Either hiding (left, right)

main = pure unit"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(parsed.module.unwrap().name, "Test");
        assert_eq!(parsed.imports.len(), 3);
        assert_eq!(parsed.imports[0].module_name, "Prelude");
        assert_eq!(parsed.imports[1].module_name, "Data.Maybe");
        assert_eq!(parsed.imports[2].module_name, "Data.Either");
    }

    // Test cases for performance edge cases
    #[test]
    fn test_very_long_import_list() {
        let mut input = String::from("import Data.Maybe (");
        for i in 0..1000 {
            if i > 0 {
                input.push_str(", ");
            }
            input.push_str(&format!("item{}", i));
        }
        input.push_str(")");

        let result = parse_import_statement(&input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_nested_comments() {
        let input = "import Data.Maybe {- {- nested -} -}";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_multiple_line_comments() {
        let input = "import Data.Maybe -- comment 1\n-- comment 2";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Maybe");
    }

    #[test]
    fn test_import_with_unicode_symbols() {
        let input = "import Data.Number (abs, (~=), (≅), (≇))";
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Number");
    }

    #[test]
    fn test_multiline_import_with_unicode_symbols() {
        let input = r#"import Data.Number (
  abs,
  (~=),
  (≅),
  (≇)
)"#;
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Number");
    }

    #[test]
    fn test_real_world_unicode_import() {
        // This is the exact content that was causing the panic
        let input = r#"import Data.Number (
  Fraction(..)
  , eqRelative
  , eqApproximate
  , (~=)
  , (≅)
  , neqApproximate
  , (≇)
  , Tolerance(..)
  , eqAbsolute
  )"#;
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Data.Number");
    }

    #[test]
    fn test_module_with_module_export() {
        let input = r#"module AssignReviews.V2.Types
  ( Action(..)
  , AssignType(..)
  , ReviewersValidationError(..)
  , IncludeIncompleteError(..)
  , AssignTypeError(..)
  , module IO
  , SubmissionLineErrorType(..)
  , SubmissionsValidationError(..)
  , AssignSpecificForm
  , AssignTab(..)
  , displayCategory
  , ConfigureParameters
  , _drawer
  , AddingReviewer(..)
  , AddingSubmission(..)
  , makeCategory
  , makeReviewerCategory
  , noCategory
  , hasCategoryMatch
  , SubmissionRemote
  , ReviewerWithCategories
  , InitialRemote
  , Category(..)
  , mkSingleCategoryInTest
  , AuthorRemote
  , State
  , Page(..)
  , ConfigureParametersForm
  , ReviewerValidationError(..)
  , isReviewerOverTheLimit
  ) where

import Prelude"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(parsed.module.unwrap().name, "AssignReviews.V2.Types");
        assert_eq!(parsed.imports.len(), 1);
        assert_eq!(parsed.imports[0].module_name, "Prelude");
    }

    #[test]
    fn test_simple_long_import() {
        let input = r#"import Type.Data.Peano.Int (class Inverse, class IsInt, class IsZeroInt, class ParseInt, class ProductInt, class SumInt, N0, N1, N10, N100, N11, N12, N13, N14, N15, N16, N17, N18, N19, N2, N20, N21, N22, N23, N24, N25, N26, N27, N28, N29, N3, N30, N31, N32, N33, N34, N35, N36, N37, N38, N39, N4, N40, N41, N42, N43, N44, N45, N46, N47, N48, N49, N5, N50, N51, N52, N53, N54, N55, N56, N57, N58, N59, N6, N60, N61, N62, N63, N64, N65, N66, N67, N68, N69, N7, N70, N71, N72, N73, N74, N75, N76, N77, N78, N79, N8, N80, N81, N82, N83, N84, N85, N86, N87, N88, N89, N9, N90, N91, N92, N93, N94, N95, N96, N97, N98, N99, Neg, P0, P1, P10, P100, P11, P12, P13, P14, P15, P16, P17, P18, P19, P2, P20, P21, P22, P23, P24, P25, P26, P27, P28, P29, P3, P30, P31, P32, P33, P34, P35, P36, P37, P38, P39, P4, P40, P41, P42, P43, P44, P45, P46, P47, P48, P49, P5, P50, P51, P52, P53, P54, P55, P56, P57, P58, P59, P6, P60, P61, P62, P63, P64, P65, P66, P67, P68, P69, P7, P70, P71, P72, P73, P74, P75, P76, P77, P78, P79, P8, P80, P81, P82, P83, P84, P85, P86, P87, P88, P89, P9, P90, P91, P92, P93, P94, P95, P96, P97, P98, P99, Pos, n0, n1, n10, n100, n11, n12, n13, n14, n15, n16, n17, n18, n19, n2, n20, n21, n22, n23, n24, n25, n26, n27, n28, n29, n3, n30, n31, n32, n33, n34, n35, n36, n37, n38, n39, n4, n40, n41, n42, n43, n44, n45, n46, n47, n48, n49, n5, n50, n51, n52, n53, n54, n55, n56, n57, n58, n59, n6, n60, n61, n62, n63, n64, n65, n66, n67, n68, n69, n7, n70, n71, n72, n73, n74, n75, n76, n77, n78, n79, n8, n80, n81, n82, n83, n84, n85, n86, n87, n88, n89, n9, n90, n91, n92, n93, n94, n95, n96, n97, n98, n99, p0, p1, p10, p100, p11, p12, p13, p14, p15, p16, p17, p18, p19, p2, p20, p21, p22, p23, p24, p25, p26, p27, p28, p29, p3, p30, p31, p32, p33, p34, p35, p36, p37, p38, p39, p4, p40, p41, p42, p43, p44, p45, p46, p47, p48, p49, p5, p50, p51, p52, p53, p54, p55, p56, p57, p58, p59, p6, p60, p61, p62, p63, p64, p65, p66, p67, p68, p69, p7, p70, p71, p72, p73, p74, p75, p76, p77, p78, p79, p8, p80, p81, p82, p83, p84, p85, p86, p87, p88, p89, p9, p90, p91, p92, p93, p94, p95, p96, p97, p98, p99, parseInt, plus, prod, reflectInt, showInt, Int, IProxy(..))"#;
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Type.Data.Peano.Int");
    }

    #[test]
    fn test_simple_two_imports() {
        let input = r#"import Prim hiding (Int(..))
import Type.Data.Peano.Int (class Inverse, class IsInt, class IsZeroInt, class ParseInt, class ProductInt, class SumInt, N0, N1, N10, N100, N11, N12, N13, N14, N15, N16, N17, N18, N19, N2, N20, N21, N22, N23, N24, N25, N26, N27, N28, N29, N3, N30, N31, N32, N33, N34, N35, N36, N37, N38, N39, N4, N40, N41, N42, N43, N44, N45, N46, N47, N48, N49, N5, N50, N51, N52, N53, N54, N55, N56, N57, N58, N59, N6, N60, N61, N62, N63, N64, N65, N66, N67, N68, N69, N7, N70, N71, N72, N73, N74, N75, N76, N77, N78, N79, N8, N80, N81, N82, N83, N84, N85, N86, N87, N88, N89, N9, N90, N91, N92, N93, N94, N95, N96, N97, N98, N99, Neg, P0, P1, P10, P100, P11, P12, P13, P14, P15, P16, P17, P18, P19, P2, P20, P21, P22, P23, P24, P25, P26, P27, P28, P29, P3, P30, P31, P32, P33, P34, P35, P36, P37, P38, P39, P4, P40, P41, P42, P43, P44, P45, P46, P47, P48, P49, P5, P50, P51, P52, P53, P54, P55, P56, P57, P58, P59, P6, P60, P61, P62, P63, P64, P65, N66, N67, N68, N69, N7, N70, N71, N72, N73, N74, N75, N76, N77, N78, N79, N8, N80, N81, N82, N83, N84, N85, N86, N87, N88, N89, N9, N90, N91, N92, N93, N94, N95, N96, N97, N98, N99, Pos, n0, n1, n10, n100, n11, n12, n13, n14, n15, n16, n17, n18, n19, n2, n20, n21, n22, n23, n24, n25, n26, n27, n28, n29, n3, n30, n31, n32, n33, n34, n35, n36, n37, n38, n39, n4, n40, n41, n42, n43, n44, n45, n46, n47, n48, n49, n5, n50, n51, n52, n53, n54, n55, n56, n57, n58, n59, n6, n60, n61, n62, n63, n64, n65, n66, n67, n68, n69, n7, n70, n71, n72, n73, n74, n75, n76, n77, n78, n79, n8, n80, n81, n82, n83, n84, n85, n86, n87, n88, n89, n9, n90, n91, n92, n93, n94, n95, n96, n97, n98, n99, p0, p1, p10, p100, p11, p12, p13, p14, p15, p16, p17, p18, p19, p2, p20, p21, p22, p23, p24, p25, p26, p27, p28, p29, p3, p30, p31, p32, p33, p34, p35, p36, p37, p38, p39, p4, p40, p41, p42, p43, p44, p45, p46, p47, p48, p49, p5, p50, p51, p52, p53, p54, p55, p56, p57, p58, p59, p6, p60, p61, p62, p63, p64, p65, p66, p67, p68, p69, p7, p70, p71, p72, n73, n74, n75, n76, n77, n78, n79, n8, n80, n81, n82, n83, n84, n85, n86, n87, n88, n89, n9, n90, n91, n92, n93, n94, n95, n96, n97, n98, n99, p0, p1, p10, p100, p11, p12, p13, p14, p15, p16, p17, p18, p19, p2, p20, p21, p22, p23, p24, p25, p26, p27, p28, p29, p3, p30, p31, p32, p33, p34, p35, p36, p37, p38, p39, p4, p40, p41, p42, p43, p44, p45, p46, p47, p48, p49, p5, p50, p51, p52, p53, p54, p55, p56, p57, p58, p59, p6, p60, p61, p62, p63, p64, p65, p66, p67, p68, p69, p7, p70, p71, p72, p73, p74, p75, p76, p77, p78, p79, p8, p80, p81, p82, p83, p84, p85, p86, p87, p88, p89, p9, p90, p91, p92, p93, p94, p95, p96, p97, p98, p99, parseInt, plus, prod, reflectInt, showInt, Int, IProxy(..))"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert_eq!(parsed.imports.len(), 2);
        assert_eq!(parsed.imports[0].module_name, "Prim");
        assert_eq!(parsed.imports[1].module_name, "Type.Data.Peano.Int");
    }

    #[test]
    fn test_two_imports_with_long_first() {
        let input = r#"import Type.Data.Peano.Int (class Inverse, class IsInt, class IsZeroInt, class ParseInt, class ProductInt, class SumInt, N0, N1, N10, N100, N11, N12, N13, N14, N15, N16, N17, N18, N19, N2, N20, N21, N22, N23, N24, N25, N26, N27, N28, N29, N3, N30, N31, N32, N33, N34, N35, N36, N37, N38, N39, N4, N40, N41, N42, N43, N44, N45, N46, N47, N48, N49, N5, N50, N51, N52, N53, N54, N55, N56, N57, N58, N59, N6, N60, N61, N62, N63, N64, N65, N66, N67, N68, N69, N7, N70, N71, N72, N73, N74, N75, N76, N77, N78, N79, N8, N80, N81, N82, N83, N84, N85, N86, N87, N88, N89, N9, N90, N91, N92, N93, N94, N95, N96, N97, N98, N99, Neg, P0, P1, P10, P100, P11, P12, P13, P14, P15, P16, P17, P18, P19, P2, P20, P21, P22, P23, P24, P25, P26, P27, P28, P29, P3, P30, P31, P32, P33, P34, P35, P36, P37, P38, P39, P4, P40, P41, P42, P43, P44, P45, P46, P47, P48, P49, P5, P50, P51, P52, P53, P54, P55, P56, P57, P58, P59, P6, P60, P61, P62, P63, P64, P65, N66, N67, N68, N69, N7, N70, N71, N72, N73, N74, N75, N76, N77, N78, N79, N8, N80, N81, N82, N83, N84, N85, N86, N87, N88, N89, N9, N90, N91, N92, N93, N94, N95, N96, N97, N98, N99, Pos, n0, n1, n10, n100, n11, n12, n13, n14, n15, n16, n17, n18, n19, n2, n20, n21, n22, n23, n24, n25, n26, n27, n28, n29, n3, n30, n31, n32, n33, n34, n35, n36, n37, n38, n39, n4, n40, n41, n42, n43, n44, n45, n46, n47, n48, n49, n5, n50, n51, n52, n53, n54, n55, n56, n57, n58, n59, n6, n60, n61, n62, n63, n64, n65, n66, n67, n68, n69, n7, n70, n71, n72, n73, n74, n75, n76, n77, n78, n79, n8, n80, n81, n82, n83, n84, n85, n86, n87, n88, n89, n9, n90, n91, n92, n93, n94, n95, n96, n97, n98, n99, p0, p1, p10, p100, p11, p12, p13, p14, p15, p16, p17, p18, p19, p2, p20, p21, p22, p23, p24, p25, p26, p27, p28, p29, p3, p30, p31, p32, p33, p34, p35, p36, p37, p38, p39, p4, p40, p41, p42, p43, p44, p45, p46, p47, p48, p49, p5, p50, p51, p52, p53, p54, p55, p56, p57, p58, p59, p6, p60, p61, p62, p63, p64, p65, p66, p67, p68, p69, p7, p70, p71, p72, p73, p74, p75, p76, p77, p78, p79, p8, p80, p81, p82, p83, p84, p85, p86, p87, p88, p89, p9, p90, p91, p92, p93, p94, p95, p96, p97, p98, p99, parseInt, plus, prod, reflectInt, showInt, Int, IProxy(..))
import Type.Data.Peano.Nat (class CompareNat, class IsNat, class IsZeroNat, class ParseNat, class ProductNat, class SumNat, class ExponentiationNat, D0, D1, D10, D100, D11, D12, D13, D14, D15, D16, D17, D18, D19, D2, D20, D21, D22, D23, D24, D25, D26, D27, D28, D29, D3, D30, D31, D32, D33, D34, D35, D36, D37, D38, D39, D4, D40, D41, D42, D43, D44, D45, D46, D47, D48, D49, D5, D50, D51, D52, D53, D54, D55, D56, D57, D58, D59, D6, D60, D61, D62, D63, D64, D65, D66, D67, D68, D69, D7, D70, D71, D72, D73, D74, D75, D76, D77, D78, D79, D8, D80, D81, D82, D83, D84, D85, D86, D87, D88, D89, D9, D90, D91, D92, D93, D94, D95, D96, D97, D98, D99, Succ, Z, d0, d1, d10, d100, d11, d12, d13, d14, d15, d16, d17, d18, d19, d2, d20, d21, d22, d23, d24, d25, d26, d27, d28, d29, d3, d30, d31, d32, d33, d34, d35, d36, d37, d38, d39, d4, d40, d41, d42, d43, d44, d45, d46, d47, d48, d49, d5, d50, d51, d52, d53, d54, d55, d56, d57, d58, d59, d6, d60, d61, d62, d63, d64, d65, d66, d67, d68, d69, d7, d70, d71, d72, d73, d74, d75, d76, d77, d78, d79, d8, d80, d81, d82, d83, d84, d85, d86, d87, d88, d89, d9, d90, d91, d92, d93, d94, d95, d96, d97, d98, d99, mulNat, parseNat, plusNat, powNat, reflectNat, showNat, Nat, NProxy(..))"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert_eq!(parsed.imports.len(), 2);
        assert_eq!(parsed.imports[0].module_name, "Type.Data.Peano.Int");
        assert_eq!(parsed.imports[1].module_name, "Type.Data.Peano.Nat");
    }

    #[test]
    fn test_simple_import_with_alias() {
        let input = r#"import OaFormless as F"#;
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "OaFormless");
    }

    #[test]
    fn test_module_import_with_alias() {
        let input = r#"import Record.Studio as RS"#;
        let result = parse_import_statement(input);
        assert!(result.is_ok());
        let (_, import) = result.unwrap();
        assert_eq!(import.module_name, "Record.Studio");
    }

    #[test]
    fn test_imports_with_parentheses() {
        let input = r#"import GraphQL.Client.Operation (OpMutation, OpQuery)
import GraphQL.Client.Query (getFullRes)
import GraphQL.Client.Query as GqlClient
import GraphQL.Client.Types (class GqlQuery, class QueryClient, Client, GqlResJson(GqlResJson))
import Record.Studio (class SingletonRecord)
import Record.Studio as RS"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        println!("All imports found:");
        for import in &parsed.imports {
            println!("  - {}", import.module_name);
        }
        assert_eq!(parsed.imports.len(), 6);
    }

    #[test]
    fn test_problematic_imports() {
        let input = r#"import GraphQL.Client.GqlError as Client.GqlError
import GraphQL.Client.Operation (OpMutation, OpQuery)
import GraphQL.Client.Query (getFullRes)
import GraphQL.Client.Query as GqlClient"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        println!("All imports found:");
        for import in &parsed.imports {
            println!("  - {}", import.module_name);
        }
        assert_eq!(parsed.imports.len(), 4);
    }

    #[test]
    fn test_import_with_hiding_and_alias() {
        let input = r#"module Test where

import OaComponents.Table hiding (Input, component) as Table
import OaFeUtils.IntersectionObserver (intersection)"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(parsed.module.unwrap().name, "Test");

        // Check that we have the expected number of imports
        assert_eq!(parsed.imports.len(), 2);

        // Check that both imports are present
        let has_table = parsed
            .imports
            .iter()
            .any(|imp| imp.module_name == "OaComponents.Table");
        let has_intersection_observer = parsed
            .imports
            .iter()
            .any(|imp| imp.module_name == "OaFeUtils.IntersectionObserver");
        assert!(has_table, "OaComponents.Table should be in imports");
        assert!(
            has_intersection_observer,
            "OaFeUtils.IntersectionObserver should be in imports"
        );
    }

    #[test]
    fn test_oa_fe_utils_intersection_observer_module() {
        let input = r#"module OaFeUtils.IntersectionObserver
  ( intersection
  ) where

import Prelude

import Effect (Effect)
import Halogen.Subscription (Emitter, makeEmitter)
import Web.HTML (HTMLElement)"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(
            parsed.module.unwrap().name,
            "OaFeUtils.IntersectionObserver"
        );

        // Check that we have the expected number of imports
        assert_eq!(parsed.imports.len(), 4);
    }

    #[test]
    fn test_infinite_scroll_table_module() {
        let input = r#"module OaComponents.InfiniteScrollTable
  ( Input
  , SetupCountSubscription
  , SetupRowSubscription
  , component
  , module Table
  ) where

import Prelude

import Data.Argonaut (class DecodeJson, class EncodeJson, decodeJson, encodeJson, jsonParser, stringify)
import Data.Array (delete, elemIndex, filter, findIndex, insertAt, length, mapWithIndex, reverse, slice, sortWith, (!!))
import Data.Either (Either(..))
import Data.Filterable (filterMapDefault)
import Data.Foldable (fold, foldl, for_, sum, traverse_)
import Data.FoldableWithIndex (foldlWithIndex)
import Data.Int (floor)
import Data.Maybe (Maybe(..), fromMaybe, isJust, maybe)
import Data.Ord (abs)
import Data.Set as Set
import Data.Traversable (traverse)
import Data.Tuple (Tuple(..), fst, snd)
import Effect (Effect)
import Effect.Aff.Class (class MonadAff)
import Effect.Class (class MonadEffect)
import Halogen (lift, liftEffect, unsubscribe)
import Halogen as H
import Halogen.HTML as HH
import Halogen.HTML.Properties (style)
import Halogen.HTML.Properties as HP
import Halogen.Portal (class PortalM)
import Halogen.Query.Event (eventListener)
import Halogen.Query.HalogenM (SubscriptionId)
import Halogen.Subscription as HS
import OaArrays.Update (moveTo)
import OaComponents.Table (default_optional_input)
import OaComponents.Table hiding (Input, component) as Table
import OaComponents.V1.PageUtils (errorSection)
import OaFeUtils.IntersectionObserver (intersection)
import OaFeUtils.OaHalogen as OaHalogen
import OaHtmlUtils (css, maybeElem, whenElem)
import OaIcons (smallIcon)
import OaIcons.HeroIcons.LoadingDonut as LoadingDonut
import Web.DOM.Document (toNonElementParentNode)
import Web.DOM.NodeList (toArray)
import Web.DOM.NonElementParentNode (getElementById)
import Web.DOM.ParentNode (QuerySelector(..), querySelectorAll)
import Web.HTML (window)
import Web.HTML.HTMLDocument (toDocument, toEventTarget, toParentNode)
import Web.HTML.HTMLElement (fromElement, fromNode, offsetWidth)
import Web.HTML.Window (document, localStorage)
import Web.Storage.Storage (getItem, setItem)
import Web.UIEvent.MouseEvent (MouseEvent, fromEvent, screenX)
import Web.UIEvent.MouseEvent.EventTypes (mousemove, mouseup)"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(
            parsed.module.unwrap().name,
            "OaComponents.InfiniteScrollTable"
        );

        // Check that we have the expected number of imports
        assert_eq!(parsed.imports.len(), 45);

        // Check that OaFeUtils.IntersectionObserver is in the imports
        let has_intersection_observer = parsed
            .imports
            .iter()
            .any(|imp| imp.module_name == "OaFeUtils.IntersectionObserver");
        assert!(
            has_intersection_observer,
            "OaFeUtils.IntersectionObserver should be in imports"
        );
    }

    #[test]
    fn test_oa_formless_module() {
        let input = r#"{-
MIT License

Copyright (c) 2020 Thomas Honeyman

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
-}

module OaFormless
  ( formless
  , FieldInput
  , FieldState
  , FieldAction
  , FieldResult
  , FieldOutput
  , FieldValidation
  , FieldValidationM
  , validate
  , validateM
  , FormContext
  , FormConfig
  , OptionalFormConfig
  , FormQuery(..)
  , FormState
  , FormAction
  , FormOutput(..)
  , eval
  , raise
  , FormlessAction -- don't export constructors
  , handleSubmitValidate
  , handleSubmitValidateM
  -- The below exports are classes and functions that must be exported for type
  -- inference for Formless to work, but they shouldn't be needed explicitly in
  -- user code.
  , class MkConfig
  , mkConfig
  , MkFieldState
  , class MkFieldStates
  , mkFieldStates
  , MkFieldAction
  , class MkFieldActions
  , mkFieldActions
  , MkFieldResult
  , class MkFieldResults
  , mkFieldResults
  , MkFieldOutput
  , class MkFieldOutputs
  , mkFieldOutputs
  ) where

import Prelude

import ConvertableOptions (class Defaults, defaults)
import Data.Either (Either(..), hush)
import Data.Foldable (for_)
import Data.Maybe (Maybe(..))
import Data.Symbol (class IsSymbol, reflectSymbol)
import Data.Variant (class VariantMapCases, Variant)
import Data.Variant as Variant
import Data.Variant.Internal (class VariantTraverseCases, VariantRep(..))
import Effect.Class (class MonadEffect)
import Foreign.Object (Object)
import Foreign.Object as Object
import Foreign.Object.Unsafe as Object.Unsafe
import Halogen as H
import Halogen.HTML as HH
import Heterogeneous.Folding (class FoldingWithIndex, class HFoldlWithIndex, hfoldlWithIndex)
import Heterogeneous.Mapping
  ( class HMap
  , class HMapWithIndex
  , class Mapping
  , class MappingWithIndex
  , hmap
  , hmapWithIndex
  )
import Prim.Row as Row
import Prim.RowList as RL
import Record as Record
import Record.Builder (Builder)
import Record.Builder as Builder
import Safe.Coerce (coerce)
import Type.Equality (class TypeEquals)
import Type.Equality as Type.Equality
import Type.Proxy (Proxy(..))
import Unsafe.Coerce (unsafeCoerce)
import Unsafe.Reference (unsafeRefEq)
import Web.Event.Event (Event)
import Web.Event.Event as Event
import Web.UIEvent.FocusEvent (FocusEvent)"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(parsed.module.unwrap().name, "OaFormless");

        // Check that we have the expected number of imports
        assert_eq!(parsed.imports.len(), 31);
    }

    #[test]
    fn test_configure_parameters_module() {
        let input = r#"module AssignReviews.V2.ConfigureParameters where

import Prelude

import AssignReviews.V2.EmptyState (emptyState)
import AssignReviews.V2.Shuffle (AutoAssignResponse, assignHTTP)
import AssignReviews.V2.Types
  ( AssignTypeError(..)
  , ConfigureParametersForm
  , IncludeIncompleteError(..)
  , ReviewerWithCategories
  , SubmissionRemote
  )
import CommandBar as CommandBar
import DOM.HTML.Indexed.InputType (InputType(..))
import Data.Array (catMaybes, filter, length)
import Data.Either (Either(..), blush, hush)
import Data.Foldable (all, any, for_, null, traverse_)
import Data.Id.EventId (EventId)
import Data.Id.StageId (StageId)
import Data.Int as Int
import Data.Maybe (Maybe(..), fromMaybe, isJust, maybe)
import Data.Monoid (guard)
import Data.NoQuery (NoQuery)
import Data.String.Common (trim)
import Data.Time.Duration (Milliseconds(..))
import Data.Variant (onMatch)
import Effect.Aff (delay)
import Effect.Aff.Class (class MonadAff, liftAff)
import Effect.Class (liftEffect)
import Halogen as H
import Halogen.HTML as HH
import Halogen.HTML.Events as HE
import Halogen.HTML.Properties as HP
import Halogen.HTML.Properties.ARIA as ARIA
import Halogen.Portal (class PortalM)
import Network.RemoteData (RemoteData(..), isLoading)
import OaComponents.Button.Primary as Primary
import OaComponents.Checkbox (checkboxSmall)
import OaComponents.DOM.ScrollIntoView (scrollIntoView)
import OaComponents.DOM.Selectors (elementById)
import OaComponents.LazyRender as LazyRender
import OaComponents.Switch (switch)
import OaComponents.V1.Form.Input (inputFn)
import OaComponents.V1.Radio as Radio
import OaComponents.V1.Tooltip as Tooltip
import OaComponents.Warn as Warn
import OaFormless as F
import OaHtmlUtils (css, maybeElem, whenElem)
import OaIcons (mediumSmallIcon, smallIcon)
import OaIcons.HeroIcons.CheckSvg as CheckIcon
import OaIcons.HeroIcons.ExclamationCircleSvg as ExclamationIcon
import OaIcons.HeroIcons.LoadingDonut as Donut
import OaIcons.HeroIcons.QuestionMarkCircleSvg as QuestionIcon
import Record.Extra (pick)
import Type.Proxy (Proxy(..))
import Web.HTML.HTMLElement (fromElement)"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(
            parsed.module.unwrap().name,
            "AssignReviews.V2.ConfigureParameters"
        );

        // Check that OaFormless is parsed as an import
        let oa_formless_import = parsed
            .imports
            .iter()
            .find(|i| i.module_name == "OaFormless");
        assert!(oa_formless_import.is_some(), "OaFormless import not found");

        // Check that we have the expected number of imports
        assert_eq!(parsed.imports.len(), 48);
    }

    #[test]
    fn test_gql_toolkit_module() {
        let input = r#"module Gql.Toolkit
  ( GqlData
  , GqlError(..)
  , GqlMutationFn
  , GqlQueryFn
  , GqlToolkitT(..)
  , class GqlToolkit
  , decodeStrictThenLiberalUnsafe
  , expectSingleField
  , expectSingleRemote
  , expectSingleResult
  , getClient
  , liftError
  , mergeGqlRes
  , module Gql.Toolkit.GqlRes
  , mutation
  , mutation'
  , mutationCl
  , mutationWithDecoder
  , query
  , query'
  , queryCl
  , queryClWithDecoder
  , queryFailed
  , queryWithDecoder
  , toEither
  , withClient
  , withGqlToolkit
  , withGqlToolkit'
  )
  where

import Prelude

import Control.Monad.Error.Class (try, liftEither, class MonadThrow, class MonadError)
import Control.Monad.Reader.Class (asks)
import Control.Monad.Reader.Trans (ReaderT, runReaderT, mapReaderT)
import Control.Monad.Trans.Class (class MonadTrans, lift)
import Data.Argonaut (JsonDecodeError, printJsonDecodeError)
import Data.Argonaut.Core (Json)
import Data.Either (Either(..))
import Data.Foldable (class Foldable, foldl, intercalate)
import Data.Generic.Rep (class Generic)
import Data.Maybe (Maybe(Just))
import Data.Symbol (class IsSymbol, reflectSymbol)
import Data.Tuple (Tuple)
import Effect.Aff.Class (class MonadAff, liftAff)
import Effect.Class (class MonadEffect)
import Effect.Exception (Error, error, message)
import Effect.Unsafe (unsafePerformEffect)
import Gql.Toolkit.GqlRes (gqlRes)
import Gql.Toolkit.UnsequenceRecord (class UnsequenceRemoteData, unsequenceRemoteData)
import GraphQL.Client.GqlError as Client.GqlError
import GraphQL.Client.Operation (OpMutation, OpQuery)
import GraphQL.Client.Query (getFullRes)
import GraphQL.Client.Query as GqlClient
import GraphQL.Client.Types (class GqlQuery, class QueryClient, Client, GqlResJson(GqlResJson))
import GraphQL.Hasura.DecodeLiberal (class DecodeHasuraLiberal, decodeLiberal, decodeStrict)
import Network.RemoteData (RemoteData(..))
import OaFeLogging.Store.Global as Global
import Prim.Row (class Nub, class Union)
import Record as Record
import Record.Studio (class SingletonRecord)
import Record.Studio as RS
import Type.Data.List (List')
import Type.Proxy (Proxy)
import Control.Parallel.Class (class Parallel, parallel, sequential)
import Data.Newtype (class Newtype, wrap, unwrap)
import Data.Bifunctor (lmap)
import Data.Function (on)"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(parsed.module.unwrap().name, "Gql.Toolkit");

        // Check that Record.Studio is parsed as an import (both with and without alias)
        let record_studio_imports: Vec<_> = parsed
            .imports
            .iter()
            .filter(|i| i.module_name == "Record.Studio")
            .collect();
        assert_eq!(
            record_studio_imports.len(),
            2,
            "Expected 2 Record.Studio imports"
        );

        // Check that we have the expected number of imports
        assert_eq!(parsed.imports.len(), 37);
    }

    #[test]
    fn test_assign_reviews_module() {
        let input = r#"module AssignReviews.V2.Types
  ( Action(..)
  , AssignType(..)
  , ReviewersValidationError(..)
  , IncludeIncompleteError(..)
  , AssignTypeError(..)
  , module IO
  , SubmissionLineErrorType(..)
  , SubmissionsValidationError(..)
  , AssignSpecificForm
  , AssignTab(..)
  , displayCategory
  , ConfigureParameters
  , _drawer
  , AddingReviewer(..)
  , AddingSubmission(..)
  , makeCategory
  , makeReviewerCategory
  , noCategory
  , hasCategoryMatch
  , SubmissionRemote
  , ReviewerWithCategories
  , InitialRemote
  , Category(..)
  , mkSingleCategoryInTest
  , AuthorRemote
  , State
  , Page(..)
  , ConfigureParametersForm
  , ReviewerValidationError(..)
  , isReviewerOverTheLimit
  ) where

import Prelude

import AssignReviews.IO (Input, Output(..), Query(..)) as IO
import Data.Argonaut.Decode.Class (decodeJson)
import Data.Argonaut.Decode.Parser (parseJson)
import Data.Array (any)
import Data.Date (Date)
import Data.Either (Either(Right))
import Data.Foldable (fold)
import Data.Generic.Rep (class Generic)
import Data.Id.AutoAssignRunId (AutoAssignRunId)
import Data.Id.EventId (EventId)
import Data.Id.StageId (StageId)
import Data.Id.SubmissionId (SubmissionId)
import Data.Id.SubmissionSerial (SubmissionSerial)
import Data.Id.UserId (UserId)
import Data.Map (Map)
import Data.Maybe (Maybe)
import Data.Set (Set)
import Data.Show.Generic (genericShow)
import Data.Tuple.Nested (type (/\))
import Effect.Exception (Error)
import Gql.Halogen.AdminDashboard (HasuraClient)
import Halogen.Subscription as HS
import Network.RemoteData (RemoteData)
import OaComponents.V1.Toasts as Toasts
import OaEnumsPostgres.PaypalPaymentCurrency (PaypalPaymentCurrency)
import OaEnumsPostgres.PricePackage (PricePackage)
import OaEnumsPostgres.QuestionDataTypes (QuestionDataTypes)
import OaEnumsPostgres.QuestionDataTypes as QuestionDataTypes
import OaFormless as F
import Type.Proxy (Proxy(..))"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(parsed.module.unwrap().name, "AssignReviews.V2.Types");

        // Check that OaFormless is parsed as an import
        let oa_formless_import = parsed
            .imports
            .iter()
            .find(|i| i.module_name == "OaFormless");
        assert!(oa_formless_import.is_some(), "OaFormless import not found");

        // Check that we have the expected number of imports
        assert_eq!(parsed.imports.len(), 31);
    }

    #[test]
    fn test_peano_module_with_long_imports() {
        let input = r#"module Type.Data.Peano
  ( module Type.Data.Peano.Int
  , module Type.Data.Peano.Nat
  ) where

import Prim hiding (Int(..))
import Type.Data.Peano.Int (class Inverse, class IsInt, class IsZeroInt, class ParseInt, class ProductInt, class SumInt, N0, N1, N10, N100, N11, N12, N13, N14, N15, N16, N17, N18, N19, N2, N20, N21, N22, N23, N24, N25, N26, N27, N28, N29, N3, N30, N31, N32, N33, N34, N35, N36, N37, N38, N39, N4, N40, N41, N42, N43, N44, N45, N46, N47, N48, N49, N5, N50, N51, N52, N53, N54, N55, N56, N57, N58, N59, N6, N60, N61, N62, N63, N64, N65, N66, N67, N68, N69, N7, N70, N71, N72, N73, N74, N75, N76, N77, N78, N79, N8, N80, N81, N82, N83, N84, N85, N86, N87, N88, N89, N9, N90, N91, N92, N93, N94, N95, N96, N97, N98, N99, Neg, P0, P1, P10, P100, P11, P12, P13, P14, P15, P16, P17, P18, P19, P2, P20, P21, P22, P23, P24, P25, P26, P27, P28, P29, P3, P30, P31, P32, P33, P34, P35, P36, P37, P38, P39, P4, P40, P41, P42, P43, P44, P45, P46, P47, P48, P49, P5, P50, P51, P52, P53, P54, P55, P56, P57, P58, P59, P6, P60, P61, P62, P63, P64, P65, P66, P67, P68, P69, P7, P70, P71, P72, P73, P74, P75, P76, P77, P78, P79, P8, P80, P81, P82, P83, P84, P85, P86, P87, P88, P89, P9, P90, P91, P92, P93, P94, P95, P96, P97, P98, P99, Pos, n0, n1, n10, n100, n11, n12, n13, n14, n15, n16, n17, n18, n19, n2, n20, n21, n22, n23, n24, n25, n26, n27, n28, n29, n3, n30, n31, n32, n33, n34, n35, n36, n37, n38, n39, n4, n40, n41, n42, n43, n44, n45, n46, n47, n48, n49, n5, n50, n51, n52, n53, n54, n55, n56, n57, n58, n59, n6, n60, n61, n62, n63, n64, n65, n66, n67, n68, n69, n7, n70, n71, n72, n73, n74, n75, n76, n77, n78, n79, n8, n80, n81, n82, n83, n84, n85, n86, n87, n88, n89, n9, n90, n91, n92, n93, n94, n95, n96, n97, n98, n99, p0, p1, p10, p100, p11, p12, p13, p14, p15, p16, p17, p18, p19, p2, p20, p21, p22, p23, p24, p25, p26, p27, p28, p29, p3, p30, p31, p32, p33, p34, p35, p36, p37, p38, p39, p4, p40, p41, p42, p43, p44, p45, p46, p47, p48, p49, p5, p50, p51, p52, p53, p54, p55, p56, p57, p58, p59, p6, p60, p61, p62, p63, p64, p65, p66, p67, p68, p69, p7, p70, p71, p72, p73, p74, p75, p76, p77, p78, p79, p8, p80, p81, p82, p83, p84, p85, p86, p87, p88, p89, p9, p90, p91, p92, p93, p94, p95, p96, p97, p98, p99, parseInt, plus, prod, reflectInt, showInt, Int, IProxy(..))
import Type.Data.Peano.Nat (class CompareNat, class IsNat, class IsZeroNat, class ParseNat, class ProductNat, class SumNat, class ExponentiationNat, D0, D1, D10, D100, D11, D12, D13, D14, D15, D16, D17, D18, D19, D2, D20, D21, D22, D23, D24, D25, D26, D27, D28, D29, D3, D30, D31, D32, D33, D34, D35, D36, D37, D38, D39, D4, D40, D41, D42, D43, D44, D45, D46, D47, D48, D49, D5, D50, D51, D52, D53, D54, D55, D56, D57, D58, D59, D6, D60, D61, D62, D63, D64, D65, D66, D67, D68, D69, D7, D70, D71, D72, D73, D74, D75, D76, D77, D78, D79, D8, D80, D81, D82, D83, D84, D85, D86, D87, D88, D89, D9, D90, D91, D92, D93, D94, D95, D96, D97, D98, D99, Succ, Z, d0, d1, d10, d100, d11, d12, d13, d14, d15, d16, d17, d18, d19, d2, d20, d21, d22, d23, d24, d25, d26, d27, d28, d29, d3, d30, d31, d32, d33, d34, d35, d36, d37, d38, d39, d4, d40, d41, d42, d43, d44, d45, d46, d47, d48, d49, d5, d50, d51, d52, d53, d54, d55, d56, d57, d58, d59, d6, d60, d61, d62, d63, d64, d65, d66, d67, d68, d69, d7, d70, d71, d72, d73, d74, d75, d76, d77, d78, d79, d8, d80, d81, d82, d83, d84, d85, d86, d87, d88, d89, d9, d90, d91, d92, d93, d94, d95, d96, d97, d98, d99, mulNat, parseNat, plusNat, powNat, reflectNat, showNat, Nat, NProxy(..))"#;
        let result = parse_purescript_file(input);
        assert!(result.is_ok());
        let (_, parsed) = result.unwrap();
        assert!(parsed.module.is_some());
        assert_eq!(parsed.module.unwrap().name, "Type.Data.Peano");
        assert_eq!(parsed.imports.len(), 3);
        assert_eq!(parsed.imports[0].module_name, "Prim");
        assert_eq!(parsed.imports[1].module_name, "Type.Data.Peano.Int");
        assert_eq!(parsed.imports[2].module_name, "Type.Data.Peano.Nat");
    }
}
