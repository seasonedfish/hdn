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
use crate::nix_parse::{findattr, getcfgbase};
use rnix::{SyntaxKind, SyntaxNode};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum ReadError {
    #[error("Error while parsing")]
    ParseError,
    #[error("No attributes")]
    NoAttr,
    #[error("Error with array")]
    ArrayError,
}

pub(crate) fn getarrvals(f: &str, query: &str) -> Result<Vec<String>, ReadError> {
    let ast = rnix::Root::parse(f);
    let configbase = match getcfgbase(&ast.syntax()) {
        Some(x) => x,
        None => {
            return Err(ReadError::ParseError);
        }
    };
    let output = match findattr(&configbase, query) {
        Some(x) => match getarrvals_aux(&x) {
            Some(y) => y,
            None => return Err(ReadError::ArrayError),
        },
        None => return Err(ReadError::NoAttr),
    };
    Ok(output)
}

fn getarrvals_aux(node: &SyntaxNode) -> Option<Vec<String>> {
    for child in node.children() {
        if child.kind() == rnix::SyntaxKind::NODE_WITH {
            return getarrvals_aux(&child);
        }
        if child.kind() == SyntaxKind::NODE_LIST {
            let mut out = vec![];
            for elem in child.children() {
                out.push(elem.to_string());
            }
            return Some(out);
        }
    }
    None
}
