/*
 * ISC License
 *
 * Copyright (c) 2021 Mitama Lab
 *
 * Permission to use, copy, modify, and/or distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 *
 */

use anyhow::anyhow;
use combine::{choice, parser, unexpected_any, value, ParseError, Parser, Stream};
use itertools::Itertools;
use parser::char::{char, letter};
use rustomaton::{automaton::Buildable, nfa::NFA};
use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
    vec::Vec,
};
use strum_macros::EnumIter;

#[derive(EnumIter, Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Alphabet {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
}

impl Alphabet {
    fn from_char(input: &char) -> anyhow::Result<Alphabet> {
        match input {
            'a' | 'A' => Ok(Alphabet::A),
            'b' | 'B' => Ok(Alphabet::B),
            'c' | 'C' => Ok(Alphabet::C),
            'd' | 'D' => Ok(Alphabet::D),
            'e' | 'E' => Ok(Alphabet::E),
            'f' | 'F' => Ok(Alphabet::F),
            'g' | 'G' => Ok(Alphabet::G),
            'h' | 'H' => Ok(Alphabet::H),
            'i' | 'I' => Ok(Alphabet::I),
            'j' | 'J' => Ok(Alphabet::J),
            _ => Err(anyhow!("Character {} is not a valid Alphabet", input)),
        }
    }

    pub fn vec_from_str(string: &str) -> anyhow::Result<Vec<Alphabet>> {
        string
            .chars()
            .map(|c| Self::from_char(&c))
            .collect::<anyhow::Result<Vec<_>>>()
    }

    pub fn slice_to_plain_string(alphabets: &[Alphabet]) -> String {
        alphabets.iter().map(|a| format!("{}", a)).join("")
    }
}

impl Display for Alphabet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Alphabet::A => write!(f, "a"),
            Alphabet::B => write!(f, "b"),
            Alphabet::C => write!(f, "c"),
            Alphabet::D => write!(f, "d"),
            Alphabet::E => write!(f, "e"),
            Alphabet::F => write!(f, "f"),
            Alphabet::G => write!(f, "g"),
            Alphabet::H => write!(f, "h"),
            Alphabet::I => write!(f, "i"),
            Alphabet::J => write!(f, "j"),
        }
    }
}

/// An abstract syntax tree of a regular expression
/// which denotes a nonempty language over [Alphabet].
///
/// In our problem domain, we do not care about empty languages since setting ∅ as the answer for a quiz
/// is very uninteresting. We therefore restrict ourselves in nonempty regular languages,
/// and the class of regular expressions corresponding to this language class will not require ∅ as a
/// constant symbol. The proof is by a simple induction over set of regular expressions.
///
/// In a string representation of this datatype, Epsilon is mapped to a character `ε`
/// and literals are mapped to either upper-case or lower-case of corresponding alphabets
/// (`fmt` method will format literals to lower-cases).
/// Star will be denoted by the postfix operator `*`,
/// alternations will be the infix operator `|` and concatenations will have no symbols.
///
/// The precedence of operators should be:
/// `Star`, `Concatenation` and then `Alternation`
/// in a descending order.
///
/// For example, `ab*|cd` should be equivalent to `(a((b)*))|(cd)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegexAst {
    /// The expression that matches the empty string
    Epsilon,
    /// An expression that matches an alphabetic literal
    Literal(Alphabet),
    /// An expression that matches a repetition of words matching inner expression
    Star(Box<RegexAst>),
    /// An expression that matches if all expressions match successively
    Concatenation(Vec<RegexAst>),
    /// An expression that matches if one of expressions matches
    Alternation(Vec<RegexAst>),
}

fn regex_parser_<Input>() -> impl Parser<Input, Output = RegexAst>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let parse_epsilon = parser::char::string("ε").map(|_s| RegexAst::Epsilon);

    let parse_literal = letter().then(|letter| match Alphabet::from_char(&letter) {
        Ok(a) => value(RegexAst::Literal(a)).left(),
        Err(_) => unexpected_any(letter).message("Unexpected literal").right(),
    });

    let parse_epsilon_literal_or_parens = choice!(
        parse_epsilon,
        parse_literal,
        char('(').with(regex_parser()).skip(char(')'))
    );

    let parse_repetitions = parse_epsilon_literal_or_parens.then(|ast| {
        combine::many::<Vec<_>, _, _>(char('*')).map(move |reps| {
            if !reps.is_empty() {
                RegexAst::Star(Box::new(ast.clone()))
            } else {
                ast.clone()
            }
        })
    });

    let parse_concat = combine::many1::<Vec<_>, _, _>(parse_repetitions).map(|asts| {
        if asts.len() > 1 {
            RegexAst::Concatenation(asts)
        } else {
            asts.first().unwrap().clone()
        }
    });

    combine::sep_by1::<Vec<_>, _, _, _>(parse_concat, char('|')).map(|asts| {
        if asts.len() > 1 {
            RegexAst::Alternation(asts)
        } else {
            asts.first().unwrap().clone()
        }
    })
}

// We need to tie the knot using `parser!` macro. See
// https://docs.rs/combine/4.6.1/combine/#examples for details.
parser! {
    fn regex_parser[Input]()(Input) -> RegexAst
    where [Input: Stream<Token = char>]
    {
        regex_parser_()
    }
}

impl RegexAst {
    pub fn parse_str(string: &str) -> anyhow::Result<RegexAst> {
        let (ast, remaining) = regex_parser().parse(string)?;
        if remaining.is_empty() {
            Ok(ast)
        } else {
            Err(anyhow!(
                r#"Failed to parse a tail "{}" of the input"#,
                remaining
            ))
        }
    }

    /// Compile the current AST to a regular expression that does not use a ε.
    fn compile_to_epsilonless_regex(&self) -> String {
        fn join_with_separator(sep: &str, asts: &[RegexAst]) -> String {
            asts.iter()
                .map(|ast| ast.compile_to_epsilonless_regex())
                .join(sep)
        }

        match self {
            RegexAst::Epsilon => "(.{0})".to_owned(),
            RegexAst::Literal(a) => format!("{}", a),
            RegexAst::Star(ast) => format!("({})*", (*ast).compile_to_epsilonless_regex()),
            RegexAst::Concatenation(asts) => format!("({})", join_with_separator("", asts)),
            RegexAst::Alternation(asts) => format!("({})", join_with_separator("|", asts)),
        }
    }

    pub fn compile_to_string_regex(&self) -> regex::Regex {
        let regex = format!("^({})$", self.compile_to_epsilonless_regex());

        regex::Regex::new(&regex).unwrap()
    }

    pub fn matches(&self, input: &[Alphabet]) -> bool {
        self.compile_to_string_regex()
            .is_match(&Alphabet::slice_to_plain_string(input))
    }

    fn compile_to_nfa(&self, alphabets: HashSet<Alphabet>) -> NFA<Alphabet> {
        match self {
            RegexAst::Epsilon => NFA::new_length(alphabets, 0),
            RegexAst::Literal(a) => NFA::new_matching(alphabets, &[*a]),
            RegexAst::Star(ast) => ast.compile_to_nfa(alphabets).kleene(),
            RegexAst::Concatenation(asts) => asts
                .iter()
                .map(|ast| ast.compile_to_nfa(alphabets.clone()))
                .fold1(|nfa1, nfa2| nfa1.concatenate(nfa2))
                .unwrap(),
            RegexAst::Alternation(asts) => asts
                .iter()
                .map(|ast| ast.compile_to_nfa(alphabets.clone()))
                .fold1(|nfa1, nfa2| nfa1.unite(nfa2))
                .unwrap(),
        }
    }

    /// Set of alphabets used within this AST.
    pub fn used_alphabets(&self) -> HashSet<Alphabet> {
        let mut accum = HashSet::new();
        let mut exprs_to_process = vec![self];

        while let Some(to_process) = exprs_to_process.pop() {
            match to_process {
                RegexAst::Epsilon => {}
                RegexAst::Literal(a) => {
                    accum.insert(*a);
                }
                RegexAst::Star(ast) => exprs_to_process.push(ast),
                RegexAst::Concatenation(asts) => exprs_to_process.extend(asts),
                RegexAst::Alternation(asts) => exprs_to_process.extend(asts),
            }
        }

        accum
    }

    pub fn equivalent_to(&self, another: &RegexAst) -> bool {
        let used_alphabets = self.used_alphabets();
        if used_alphabets != another.used_alphabets() {
            // Proposition: A word containing a letter α is never accepted by RegexAst `r` if
            //              r does not contain α.
            //   Proof: By a straightforward induction on `r`.
            //
            // Proposition: If a RegexAst `r` contains a literal α, then there exists a word
            //              containing α that is accepted by `r`.
            //   Proof: Base case is immediate.
            //          For inductive part, notice that RegexAst always corresponds to a
            //          nonempty language, so by case-wise analysis
            //          we can always construct such a word.
            //
            // Corollary: if two RegexAst have different set of used_alphabets, they are not equivalent.
            return false;
        }

        let nfa_1 = self.compile_to_nfa(used_alphabets.clone());
        let nfa_2 = another.compile_to_nfa(used_alphabets);

        nfa_1.eq(&nfa_2)
    }

    //region flattening oeprations

    fn flatten_alternations(&self) -> Self {
        fn apply_to_ast_vec(vec: &[RegexAst]) -> Vec<RegexAst> {
            vec.iter().map(|ast| ast.flatten_alternations()).collect()
        }

        match self {
            RegexAst::Epsilon | RegexAst::Literal(_) => self.clone(),
            RegexAst::Star(ast) => RegexAst::Star(Box::new(ast.flatten_alternations())),
            RegexAst::Concatenation(asts) => RegexAst::Concatenation(apply_to_ast_vec(asts)),
            RegexAst::Alternation(asts) if asts.len() == 1 => {
                asts.first().unwrap().flatten_alternations()
            }
            RegexAst::Alternation(asts) => RegexAst::Alternation(
                apply_to_ast_vec(asts)
                    .into_iter()
                    .flat_map(|ast| match ast {
                        RegexAst::Alternation(asts) => asts,
                        _ => vec![ast],
                    })
                    .collect(),
            ),
        }
    }

    fn flatten_consecutive_concatenations(&self) -> Self {
        fn apply_to_ast_vec(vec: &[RegexAst]) -> Vec<RegexAst> {
            vec.iter().map(|ast| ast.flatten_alternations()).collect()
        }

        match self {
            RegexAst::Epsilon | RegexAst::Literal(_) => self.clone(),
            RegexAst::Star(ast) => {
                RegexAst::Star(Box::new(ast.flatten_consecutive_concatenations()))
            }
            RegexAst::Concatenation(asts) if asts.len() == 1 => {
                asts.first().unwrap().flatten_consecutive_concatenations()
            }
            RegexAst::Concatenation(asts) => RegexAst::Concatenation(
                apply_to_ast_vec(asts)
                    .into_iter()
                    .flat_map(|ast| match ast {
                        RegexAst::Concatenation(asts) => asts,
                        _ => vec![ast],
                    })
                    .collect(),
            ),
            RegexAst::Alternation(asts) => RegexAst::Alternation(apply_to_ast_vec(asts)),
        }
    }

    fn flatten_consecutive_stars(&self) -> Self {
        fn apply_to_ast_vec(vec: &[RegexAst]) -> Vec<RegexAst> {
            vec.iter()
                .map(|ast| ast.flatten_consecutive_stars())
                .collect()
        }

        match self {
            RegexAst::Epsilon | RegexAst::Literal(_) => self.clone(),
            RegexAst::Star(ast) => {
                let flattened_child = ast.flatten_consecutive_stars();
                match flattened_child {
                    RegexAst::Star(grand_child) => *grand_child,
                    _ => RegexAst::Star(Box::new(flattened_child)),
                }
            }
            RegexAst::Concatenation(asts) => RegexAst::Concatenation(apply_to_ast_vec(asts)),
            RegexAst::Alternation(asts) => RegexAst::Alternation(apply_to_ast_vec(asts)),
        }
    }

    /// Flattens the AST.
    ///
    /// This operation applies the following transformations:
    ///
    ///  * When Alternation is a direct child of another Alternation, flatten it.
    ///    For example, `(a|(b|c))` will be flattened into `(a|b|c)`.
    ///  * When Concatenation is a direct child of another Concatenation, flatten it.
    ///    For example, `(a(bc))` will be flattened into `(abc)`.
    ///  * When Star is a direct child of another Star, flatten it.
    ///    For example, `(a*)*` will be flattened into `(a*)`.
    ///  * When Alternation contains a singleton vector, flatten it.
    ///    For example, `Alternation(vec![ab])` will be flattened into `ab`.
    ///  * When Concatenation contains a singleton vector, flatten it.
    ///    For example, `Concatenation(vec![a|b])` will be flattened into `a|b`.
    ///
    /// This operation preserves the regular expression up to equivalence.
    /// That is, [matches] returns true on the original AST if and only if
    /// it returns true on the returned AST.
    pub fn flatten(&self) -> Self {
        self.flatten_alternations()
            .flatten_consecutive_concatenations()
            .flatten_consecutive_stars()
    }

    //endregion
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum FmtPrecedence {
    Alternation,
    Concatenation,
    Star,
}

/// Convert the AST to a string with minimal usage of parentheses,
/// using the information about the operator precedence of the enclosing context.
fn show_with_precedence(prec: FmtPrecedence, ast: &RegexAst) -> String {
    match ast {
        RegexAst::Epsilon => "ε".to_owned(),
        RegexAst::Literal(a) => format!("{a}"),
        RegexAst::Star(ast) => format!("{}*", show_with_precedence(FmtPrecedence::Star, ast)),
        RegexAst::Concatenation(asts) => {
            let show_parens = prec > FmtPrecedence::Concatenation;

            let inner = asts
                .iter()
                .map(|ast| show_with_precedence(FmtPrecedence::Concatenation, ast))
                .join("");

            if show_parens {
                format!("({inner})")
            } else {
                inner
            }
        }
        RegexAst::Alternation(asts) => {
            let show_parens = prec > FmtPrecedence::Alternation;

            let inner = asts
                .iter()
                .map(|ast| show_with_precedence(FmtPrecedence::Concatenation, ast))
                .join("|");

            if show_parens {
                format!("({inner})")
            } else {
                inner
            }
        }
    }
}

impl Display for RegexAst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            show_with_precedence(FmtPrecedence::Alternation, self)
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::regex::{Alphabet, RegexAst};

    #[test]
    fn str_to_alphabets() {
        assert_eq!(
            Alphabet::vec_from_str("ABCJ").unwrap(),
            vec![Alphabet::A, Alphabet::B, Alphabet::C, Alphabet::J]
        );

        assert_eq!(
            Alphabet::vec_from_str("abcj").unwrap(),
            vec![Alphabet::A, Alphabet::B, Alphabet::C, Alphabet::J]
        );

        assert_eq!(
            Alphabet::vec_from_str("abCg").unwrap(),
            vec![Alphabet::A, Alphabet::B, Alphabet::C, Alphabet::G]
        )
    }

    #[test]
    #[should_panic]
    fn str_to_alphabets_panic() {
        Alphabet::vec_from_str("Z").unwrap();
    }

    #[test]
    fn str_to_regex_ast() {
        assert_eq!(
            RegexAst::parse_str("abc").unwrap(),
            RegexAst::Concatenation(vec![
                RegexAst::Literal(Alphabet::A),
                RegexAst::Literal(Alphabet::B),
                RegexAst::Literal(Alphabet::C),
            ])
        );

        assert_eq!(
            RegexAst::parse_str("ab|c").unwrap(),
            RegexAst::Alternation(vec![
                RegexAst::Concatenation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Literal(Alphabet::B)
                ]),
                RegexAst::Literal(Alphabet::C)
            ])
        );

        assert_eq!(
            RegexAst::parse_str("ab*|cd").unwrap(),
            RegexAst::Alternation(vec![
                RegexAst::Concatenation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Star(Box::new(RegexAst::Literal(Alphabet::B))),
                ]),
                RegexAst::Concatenation(vec![
                    RegexAst::Literal(Alphabet::C),
                    RegexAst::Literal(Alphabet::D)
                ])
            ])
        );
    }

    #[test]
    fn regex_ast_matches() {
        let positives = vec![
            ("ab|c", "ab"),
            ("ab|c", "c"),
            ("ε|a", ""),
            ("ε|a", "a"),
            ("a*bεcc*", "bc"),
            ("a*bεcc*", "aabccc"),
            ("ε", ""),
            ("ε*", ""),
        ];
        let negatives = vec![("ε|a", "ab"), ("ε|aaa*", "a"), ("a*bεcc*", "aac")];

        for (regex_str, input_str) in positives {
            let ast = RegexAst::parse_str(regex_str).unwrap();
            let input = Alphabet::vec_from_str(input_str).unwrap();
            assert!(
                ast.matches(&input),
                "The expression \"{}\" should match \"{}\"",
                regex_str,
                input_str
            )
        }

        for (regex_str, input_str) in negatives {
            let ast = RegexAst::parse_str(regex_str).unwrap();
            let input = Alphabet::vec_from_str(input_str).unwrap();
            assert!(
                !ast.matches(&input),
                "The expression \"{}\" should not match \"{}\"",
                regex_str,
                input_str
            )
        }
    }

    #[test]
    fn fmt_regex_ast() {
        assert_eq!(
            "abε",
            format!(
                "{}",
                RegexAst::Concatenation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Literal(Alphabet::B),
                    RegexAst::Epsilon,
                ])
            )
        );

        assert_eq!(
            "a|b|ε",
            format!(
                "{}",
                RegexAst::Alternation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Literal(Alphabet::B),
                    RegexAst::Epsilon,
                ])
            )
        );

        assert_eq!(
            "(a|g)*",
            format!(
                "{}",
                RegexAst::Star(Box::new(RegexAst::Alternation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Literal(Alphabet::G),
                ])))
            )
        );

        assert_eq!(
            "(a|bc)*",
            format!(
                "{}",
                RegexAst::Star(Box::new(RegexAst::Alternation(vec![
                    RegexAst::Literal(Alphabet::A),
                    RegexAst::Concatenation(vec![
                        RegexAst::Literal(Alphabet::B),
                        RegexAst::Literal(Alphabet::C),
                    ]),
                ])))
            )
        );

        assert_eq!(
            "((a|c)|bc)*",
            format!(
                "{}",
                RegexAst::Star(Box::new(RegexAst::Alternation(vec![
                    RegexAst::Alternation(vec![
                        RegexAst::Literal(Alphabet::A),
                        RegexAst::Literal(Alphabet::C)
                    ]),
                    RegexAst::Concatenation(vec![
                        RegexAst::Literal(Alphabet::B),
                        RegexAst::Literal(Alphabet::C),
                    ]),
                ])))
            )
        );
    }

    #[test]
    fn regex_ast_used_alphabets() {
        let pairs = vec![("(agb|c*)g", "abcg"), ("agb|c*g", "abcg")];

        for (regex_str, alphabets_str) in pairs {
            let ast = RegexAst::parse_str(regex_str).unwrap();
            let alphabets = Alphabet::vec_from_str(alphabets_str)
                .unwrap()
                .into_iter()
                .collect();

            assert_eq!(
                ast.used_alphabets(),
                alphabets,
                r#"Alphabets used in "{}" should be "{:?}""#,
                ast,
                alphabets
            )
        }
    }

    #[test]
    fn regex_ast_equivalence() {
        fn compile_to_regex_ast(regex_str: &str) -> RegexAst {
            RegexAst::parse_str(regex_str).unwrap()
        }

        let positives = vec![
            ("abεc", "εabc"),
            ("ε|εεε*", "ε"),
            ("(a|b)*a", "(a|b)*baa*|aa*"),
            ("(a|b|c)*(a|b)", "((a|b|c)*c(a|b)(a|b)*)|((a|b)(a|b)*)"),
            ("(a|b)*", "a*(ba*)*"),
        ];
        let negatives = vec![("abεc", "abbc"), ("ε", "a")];

        for (regex_str_1, regex_str_2) in positives {
            let ast_1 = compile_to_regex_ast(regex_str_1);
            let ast_2 = compile_to_regex_ast(regex_str_2);

            assert!(
                ast_1.equivalent_to(&ast_2),
                "The regular expression \"{}\" should be equivalent to \"{}\"",
                ast_1,
                ast_2
            )
        }

        for (regex_str_1, regex_str_2) in negatives {
            let ast_1 = compile_to_regex_ast(regex_str_1);
            let ast_2 = compile_to_regex_ast(regex_str_2);

            assert!(
                !ast_1.equivalent_to(&ast_2),
                "The regular expression \"{}\" should not be equivalent to \"{}\"",
                ast_1,
                ast_2
            )
        }
    }

    #[test]
    fn regex_ast_flattening() {
        assert_eq!(
            RegexAst::parse_str("a(b(a|(b|(c|d))))").unwrap().flatten(),
            RegexAst::parse_str("ab(a|b|c|d)").unwrap()
        );

        assert_eq!(
            RegexAst::parse_str("(((a)*)*)*").unwrap().flatten(),
            RegexAst::parse_str("a*").unwrap()
        );

        assert_eq!(
            RegexAst::Alternation(vec![RegexAst::parse_str("(((a)*)*)*").unwrap()]).flatten(),
            RegexAst::parse_str("a*").unwrap()
        );

        assert_eq!(
            RegexAst::Concatenation(vec![RegexAst::parse_str("(((a)*)*)*").unwrap()]).flatten(),
            RegexAst::parse_str("a*").unwrap()
        );
    }
}
