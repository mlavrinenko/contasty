//! Attribute-group expansion: absorb an `#[a] #[b] item` decoration run into a
//! single byte range so eliding a test/item takes its attributes with it.

use super::AstNode;

/// Given an attribute node, walk to its decorated item, absorbing any adjacent
/// attribute siblings. Returns the byte range covering the whole `#[a] #[b]
/// item` group.
pub(super) fn expand_attribute_to_item(attr: &AstNode<'_>) -> (usize, usize) {
    let mut start = attr.range().start;
    let mut cursor = attr.clone();
    while let Some(prev) = named_prev(&cursor) {
        if !is_attribute(&prev) {
            break;
        }
        start = prev.range().start;
        cursor = prev;
    }
    let mut end = attr.range().end;
    let mut cursor = attr.clone();
    while let Some(next) = named_next(&cursor) {
        end = next.range().end;
        let absorb = is_attribute(&next);
        cursor = next;
        if !absorb {
            break;
        }
    }
    (start, end)
}

fn is_attribute(node: &AstNode<'_>) -> bool {
    node.kind().as_ref() == "attribute_item"
}

fn named_prev<'r>(node: &AstNode<'r>) -> Option<AstNode<'r>> {
    let mut prev = node.prev();
    while let Some(candidate) = prev {
        if candidate.is_named() {
            return Some(candidate);
        }
        prev = candidate.prev();
    }
    None
}

fn named_next<'r>(node: &AstNode<'r>) -> Option<AstNode<'r>> {
    let mut next = node.next();
    while let Some(candidate) = next {
        if candidate.is_named() {
            return Some(candidate);
        }
        next = candidate.next();
    }
    None
}
