//! Port of the upstream tests of the string splitter (`media/string_splitter.rs`).
//!
//! The upstream test `SplitRespectingBrackets_WithoutBrackets_NullReturnsEmptyArray`
//! has no counterpart: the input cannot be null here.

use crate::media::{split_respecting_brackets, StringSplitOptions};

const DEFAULT_OPENING_PARENTHESIS: char = '(';
const DEFAULT_CLOSING_PARENTHESIS: char = ')';

fn enumerate_string_split_options_combinations() -> [StringSplitOptions; 4] {
    [
        StringSplitOptions::NONE,
        StringSplitOptions::REMOVE_EMPTY_ENTRIES,
        StringSplitOptions::TRIM_ENTRIES,
        StringSplitOptions { remove_empty_entries: true, trim_entries: true },
    ]
}

/// `string.Split(separators, options)` of the runtime of the original, which
/// the tests without brackets compare with.
fn split<'a>(input: &'a str, separators: &[char], options: StringSplitOptions) -> Vec<&'a str> {
    input
        .split(|c: char| separators.contains(&c))
        .map(|entry| if options.trim_entries { entry.trim() } else { entry })
        .filter(|entry| !options.remove_empty_entries || !entry.is_empty())
        .collect()
}

fn split_default_brackets<'a>(input: &'a str, separators: &[char], options: StringSplitOptions) -> Vec<&'a str> {
    split_respecting_brackets(input, separators, DEFAULT_OPENING_PARENTHESIS, DEFAULT_CLOSING_PARENTHESIS, options)
        .expect("matched brackets")
}

// Tests without brackets - should match string.Split behavior

#[test]
fn split_respecting_brackets_without_brackets_single_separator() {
    let cases: [(&str, char); 18] = [
        ("", ','),
        ("   ", ','),
        ("\t\n", ','),
        ("abc", ','),
        ("a,b,c", ','),
        ("a,,c", ','),
        ("a,,,b", ','),
        (",a,b,", ','),
        (" a , b , c ", ','),
        (" a ,,,, c ", ','),
        (" a ,  , c ", ','),
        (" a, b ,c ", ','),
        (" , a , b , ", ','),
        ("  a  ,  b  ,  c  ", ','),
        ("First,Second,Third", ','),
        ("Header\nBody\nFooter\n", '\n'),
        ("Width;Height;Margin;Padding", ';'),
        ("FerroUI.Utilities.StringSplitter", '.'),
    ];
    for (input, separator) in cases {
        for options in enumerate_string_split_options_combinations() {
            let result = split_default_brackets(input, &[separator], options);
            let expected = split(input, &[separator], options);
            assert_eq!(expected, result, "{input:?} {options:?}");
        }
    }
}

#[test]
fn split_respecting_brackets_without_brackets_multiple_separators() {
    let cases = ["a,b;c,d", "a,b;,;c,d", " a , b ; c , d ", " a , b ; ; c , d ", " a , b ;,; c , d ", " ; a , b ; c , d ; "];
    let separators = [',', ';'];
    for input in cases {
        for options in enumerate_string_split_options_combinations() {
            let result = split_default_brackets(input, &separators, options);
            let expected = split(input, &separators, options);
            assert_eq!(expected, result, "{input:?} {options:?}");
        }
    }
}

// Tests with brackets - should respect bracket pairs

#[test]
fn split_respecting_brackets_with_brackets_default_brackets() {
    let cases: Vec<(&str, Vec<&str>)> = vec![
        ("(a)(b,c)", vec!["(a)(b,c)"]),
        ("a,(),b", vec!["a", "()", "b"]),
        ("a,(b,c),d", vec!["a", "(b,c)", "d"]),
        ("a,(b,(c,d)),e", vec!["a", "(b,(c,d))", "e"]),
        (",a,(b,c),d,", vec!["", "a", "(b,c)", "d", ""]),
        ("(a,b),(c,d),(e,f)", vec!["(a,b)", "(c,d)", "(e,f)"]),
        ("a,(b,(c,(d,e))),f", vec!["a", "(b,(c,(d,e)))", "f"]),
        ("Button,TextBox(Width=100,Height=50),Label", vec!["Button", "TextBox(Width=100,Height=50)", "Label"]),
        ("string,List(int),Dictionary(string,object)", vec!["string", "List(int)", "Dictionary(string,object)"]),
        (
            "FirstItem,Item(param1,param2,param3),x,VeryLongItemName(a,b),Short",
            vec!["FirstItem", "Item(param1,param2,param3)", "x", "VeryLongItemName(a,b)", "Short"],
        ),
        (
            "BindingPath,Converter(Type=MyConverter,Parameter=Value123),Mode=TwoWay",
            vec!["BindingPath", "Converter(Type=MyConverter,Parameter=Value123)", "Mode=TwoWay"],
        ),
        (
            "Observable(List(Dictionary(string,int))),SimpleType,AnotherObservable(string)",
            vec!["Observable(List(Dictionary(string,int)))", "SimpleType", "AnotherObservable(string)"],
        ),
        (
            "OuterType(InnerType(DeepType(VeryDeepValue1,VeryDeepValue2),InnerValue),OuterValue)",
            vec!["OuterType(InnerType(DeepType(VeryDeepValue1,VeryDeepValue2),InnerValue),OuterValue)"],
        ),
        (
            "0 4 6 -1 #FF000000,0 2 4 -1 rgba(0,0,0,0.06),inset 0 1 2 0 rgba(255,255,255,0.1)",
            vec!["0 4 6 -1 #FF000000", "0 2 4 -1 rgba(0,0,0,0.06)", "inset 0 1 2 0 rgba(255,255,255,0.1)"],
        ),
    ];
    for (input, expected) in cases {
        let result = split_default_brackets(input, &[','], StringSplitOptions::NONE);
        assert_eq!(expected, result, "{input:?}");
    }
}

#[test]
fn split_respecting_brackets_with_brackets_multiple_separators() {
    let cases: Vec<(&str, Vec<&str>)> = vec![
        ("a,(b,c;d);e", vec!["a", "(b,c;d)", "e"]),
        (
            "Width=100,Height=200;Margin(10;20;30;40),Padding=5",
            vec!["Width=100", "Height=200", "Margin(10;20;30;40)", "Padding=5"],
        ),
    ];
    for (input, expected) in cases {
        let result = split_default_brackets(input, &[',', ';'], StringSplitOptions::NONE);
        assert_eq!(expected, result, "{input:?}");
    }
}

#[test]
fn split_respecting_brackets_with_brackets_custom_brackets() {
    let cases: Vec<(&str, char, char, Vec<&str>)> = vec![
        ("a,(b,c),d", '[', ']', vec!["a", "(b", "c)", "d"]),
        ("a,[b,c],d", '[', ']', vec!["a", "[b,c]", "d"]),
        ("x,<y,z>,w", '<', '>', vec!["x", "<y,z>", "w"]),
        ("Property1,Property2[Index1,Index2],Property3", '[', ']', vec!["Property1", "Property2[Index1,Index2]", "Property3"]),
    ];
    for (input, opening_bracket, closing_bracket, expected) in cases {
        let result =
            split_respecting_brackets(input, &[','], opening_bracket, closing_bracket, StringSplitOptions::NONE)
                .expect("matched brackets");
        assert_eq!(expected, result, "{input:?}");
    }
}

#[test]
fn split_respecting_brackets_with_brackets_with_options() {
    const NONE: StringSplitOptions = StringSplitOptions::NONE;
    const REMOVE_EMPTY_ENTRIES: StringSplitOptions = StringSplitOptions::REMOVE_EMPTY_ENTRIES;
    const TRIM_ENTRIES: StringSplitOptions = StringSplitOptions::TRIM_ENTRIES;
    const TRIM_AND_REMOVE: StringSplitOptions = StringSplitOptions { remove_empty_entries: true, trim_entries: true };

    let cases: Vec<(&str, StringSplitOptions, Vec<&str>)> = vec![
        ("a,,(b,c),,d", NONE, vec!["a", "", "(b,c)", "", "d"]),
        ("a,,(b,c),,d", REMOVE_EMPTY_ENTRIES, vec!["a", "(b,c)", "d"]),
        (",a,(b,c),d,", NONE, vec!["", "a", "(b,c)", "d", ""]),
        (",a,(b,c),d,", REMOVE_EMPTY_ENTRIES, vec!["a", "(b,c)", "d"]),
        (" a , (b, c) , d ", NONE, vec![" a ", " (b, c) ", " d "]),
        (" a , (b, c) , d ", TRIM_ENTRIES, vec!["a", "(b, c)", "d"]),
        (" a ,  , (b, c) ,  , d ", NONE, vec![" a ", "  ", " (b, c) ", "  ", " d "]),
        (" a ,  , (b, c) ,  , d ", TRIM_ENTRIES, vec!["a", "", "(b, c)", "", "d"]),
        (" a ,  , (b, c) ,  , d ", TRIM_AND_REMOVE, vec!["a", "(b, c)", "d"]),
        (" , a , ( b , ( c , d ) ) , , e , ", NONE, vec![" ", " a ", " ( b , ( c , d ) ) ", " ", " e ", " "]),
        (" , a , ( b , ( c , d ) ) , , e , ", TRIM_ENTRIES, vec!["", "a", "( b , ( c , d ) )", "", "e", ""]),
        (" , a , ( b , ( c , d ) ) , , e , ", TRIM_AND_REMOVE, vec!["a", "( b , ( c , d ) )", "e"]),
    ];
    for (input, options, expected) in cases {
        let result = split_default_brackets(input, &[','], options);
        assert_eq!(expected, result, "{input:?} {options:?}");
    }
}

// Tests for mismatched brackets - should throw exceptions

#[test]
fn split_respecting_brackets_unmatched_brackets_throws_format_exception() {
    let cases: [(&str, char, char); 13] = [
        ("(", '(', ')'),
        (")", '(', ')'),
        (")a,b(", '(', ')'),
        ("a,b),c", '(', ')'),
        ("a,(b,c", '(', ')'),
        ("a,b))c", '(', ')'),
        ("a,((b,c)", '(', ')'),
        ("a,(b,(c)),d)", '(', ')'),
        ("x,[y,z", '[', ']'),
        ("x,y],z", '[', ']'),
        ("Type1,Type2(Inner1,Inner2)),Type3", '(', ')'),
        ("Property1,Property2(Parameter1,Parameter2,Property3", '(', ')'),
        ("OuterType(InnerType(DeepType(Value1,Value2),MiddleType(Value3)", '(', ')'),
    ];
    for (input, opening_bracket, closing_bracket) in cases {
        let result =
            split_respecting_brackets(input, &[','], opening_bracket, closing_bracket, StringSplitOptions::NONE);
        assert!(result.is_err(), "{input:?}");
    }
}

#[test]
fn split_respecting_brackets_same_opening_and_closing_bracket_throws_argument_exception() {
    for (bracket1, bracket2) in [('(', '('), ('[', '['), ('.', '.')] {
        let input = "a,b,c";

        let result = std::panic::catch_unwind(|| {
            split_respecting_brackets(input, &[','], bracket1, bracket2, StringSplitOptions::NONE).map(|_| ())
        });
        assert!(result.is_err(), "{bracket1:?}");
    }
}
