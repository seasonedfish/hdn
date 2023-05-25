/*
Taken from nix-editor 0.3.0, which is licensed under the MIT License.

The MIT License (MIT)

Copyright (c) 2022 Victor Fuentes

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
 */
use crate::nix_parse::{findattr, getcfgbase, getkey};
use rnix::{self, SyntaxKind, SyntaxNode};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum WriteError {
    #[error("Error while parsing.")]
    ParseError,
    #[error("No attributes.")]
    NoAttr,
    #[error("Error with array.")]
    ArrayError,
}

fn addvalue(configbase: &SyntaxNode, query: &str, val: &str) -> SyntaxNode {
    let mut index = configbase.green().children().len() - 2;
    // To find a better index for insertion, first find a matching node, then find the next newline token, after that, insert
    if let Some(x) = matchval(configbase, query, query.split('.').count()) {
        let i = configbase
            .green()
            .children()
            .position(|y| match y.into_node() {
                Some(y) => y.to_owned() == x.green().into_owned(),
                None => false,
            })
            .unwrap();
        let configgreen = configbase.green().clone();
        let configafter = &configgreen.children().collect::<Vec<_>>()[i..];
        for child in configafter {
            if let Some(x) = child.as_token() {
                if x.text().contains('\n') {
                    let cas = configafter.to_vec();
                    index = i + cas
                        .iter()
                        .position(|y| match y.as_token() {
                            Some(t) => t == x,
                            None => false,
                        })
                        .unwrap();
                    break;
                }
            }
        }
    }
    let input = rnix::Root::parse(format!("\n  {} = {};", &query, &val).as_str()).syntax();
    let input = input.green().clone();
    if index == 0 {
        index += 1;
    };
    let new = configbase
        .green()
        .insert_child(index, rnix::NodeOrToken::Node(input.into_owned()));
    let replace = configbase.replace_with(new);
    rnix::Root::parse(&replace.to_string()).syntax()
}

fn matchval(configbase: &SyntaxNode, query: &str, acc: usize) -> Option<SyntaxNode> {
    let qvec = &query
        .split('.')
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    let q = &qvec[..acc];
    for child in configbase.children() {
        if child.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
            for subchild in child.children() {
                if subchild.kind() == SyntaxKind::NODE_ATTRPATH {
                    let key = getkey(&subchild);
                    if key.len() >= q.len() && &key[..q.len()] == q {
                        return Some(child);
                    }
                }
            }
        }
    }
    if acc == 1 {
        None
    } else {
        matchval(configbase, query, acc - 1)
    }
}

pub(crate) fn addtoarr(f: &str, query: &str, items: Vec<String>) -> Result<String, WriteError> {
    let ast = rnix::Root::parse(f);
    let configbase = match getcfgbase(&ast.syntax()) {
        Some(x) => x,
        None => return Err(WriteError::ParseError),
    };
    let outnode = match findattr(&configbase, query) {
        Some(x) => match addtoarr_aux(&x, items) {
            Some(x) => x,
            None => return Err(WriteError::ArrayError),
        },
        // If no arrtibute is found, create a new one
        None => {
            let newval = addvalue(&configbase, query, "[\n  ]");
            return addtoarr(&newval.to_string(), query, items);
        }
    };
    Ok(outnode.to_string())
}

fn addtoarr_aux(node: &SyntaxNode, items: Vec<String>) -> Option<SyntaxNode> {
    for child in node.children() {
        if child.kind() == rnix::SyntaxKind::NODE_WITH {
            return addtoarr_aux(&child, items);
        }
        if child.kind() == SyntaxKind::NODE_LIST {
            let mut green = child.green().into_owned();

            for elem in items {
                let mut i = 0;
                for c in green.children() {
                    if c.to_string() == "]" {
                        if green.children().collect::<Vec<_>>()[i - 1]
                            .as_token()
                            .unwrap()
                            .to_string()
                            .contains('\n')
                        {
                            i -= 1;
                        }
                        green = green.insert_child(
                            i,
                            rnix::NodeOrToken::Node(
                                rnix::Root::parse(&format!("\n{}{}", " ".repeat(4), elem))
                                    .syntax()
                                    .green()
                                    .into_owned(),
                            ),
                        );
                        break;
                    }
                    i += 1;
                }
            }

            let index = match node.green().children().position(|x| match x.into_node() {
                Some(x) => x.to_owned() == child.green().into_owned(),
                None => false,
            }) {
                Some(x) => x,
                None => return None,
            };

            let replace = node
                .green()
                .replace_child(index, rnix::NodeOrToken::Node(green));
            let out = node.replace_with(replace);
            let output = rnix::Root::parse(&out.to_string()).syntax();
            return Some(output);
        }
    }
    None
}

pub(crate) fn rmarr(f: &str, query: &str, items: Vec<String>) -> Result<String, WriteError> {
    let ast = rnix::Root::parse(f);
    let configbase = match getcfgbase(&ast.syntax()) {
        Some(x) => x,
        None => return Err(WriteError::ParseError),
    };
    let outnode = match findattr(&configbase, query) {
        Some(x) => match rmarr_aux(&x, items) {
            Some(x) => x,
            None => return Err(WriteError::ArrayError),
        },
        None => return Err(WriteError::NoAttr),
    };
    Ok(outnode.to_string())
}

fn rmarr_aux(node: &SyntaxNode, items: Vec<String>) -> Option<SyntaxNode> {
    for child in node.children() {
        if child.kind() == rnix::SyntaxKind::NODE_WITH {
            return rmarr_aux(&child, items);
        }
        if child.kind() == SyntaxKind::NODE_LIST {
            let green = child.green().into_owned();
            let mut idx = vec![];
            for elem in green.children() {
                if elem.as_node().is_some() && items.contains(&elem.to_string()) {
                    let index = match green.children().position(|x| match x.into_node() {
                        Some(x) => {
                            if let Some(y) = elem.as_node() {
                                x.eq(y)
                            } else {
                                false
                            }
                        }
                        None => false,
                    }) {
                        Some(x) => x,
                        None => return None,
                    };
                    idx.push(index)
                }
            }
            let mut acc = 0;
            let mut replace = green;

            for i in idx {
                replace = replace.remove_child(i - acc);
                let mut v = vec![];
                for c in replace.children() {
                    v.push(c);
                }
                if let Some(x) = v.get(i - acc - 1).unwrap().as_token() {
                    if x.to_string().contains('\n') {
                        replace = replace.remove_child(i - acc - 1);
                        acc += 1;
                    }
                }
                acc += 1;
            }
            let out = child.replace_with(replace);

            let output = rnix::Root::parse(&out.to_string()).syntax();
            return Some(output);
        }
    }
    None
}
