use super::*;
use crate::spec::FrameworkRuleBlock;

// ---------------------------------------------------------------------------
// upsert_block — insert new
// ---------------------------------------------------------------------------

#[test]
fn upsert_block_should_insert_new_block() {
    let content = "*filter\n:INPUT DROP\nCOMMIT\n";
    let block = FrameworkRuleBlock {
        id: "myapp-nat".into(),
        content: "*nat\n:POSTROUTING ACCEPT [0:0]\nCOMMIT".into(),
        ipv6: false,
    };

    let result = upsert_block(content, &block).unwrap();
    assert!(result.contains(">>> ufw-kit myapp-nat"));
    assert!(result.contains("<<< ufw-kit myapp-nat"));
    assert!(result.contains("*nat"));
}

// ---------------------------------------------------------------------------
// upsert_block — replace existing
// ---------------------------------------------------------------------------

#[test]
fn upsert_block_should_replace_existing_block() {
    let content = "\
*filter
:INPUT DROP
COMMIT
# >>> ufw-kit myapp-nat
*old-content
# <<< ufw-kit myapp-nat
";

    let block = FrameworkRuleBlock {
        id: "myapp-nat".into(),
        content: "*nat\n:POSTROUTING ACCEPT [0:0]\nCOMMIT".into(),
        ipv6: false,
    };

    let result = upsert_block(content, &block).unwrap();
    assert!(result.contains("*nat"));
    assert!(!result.contains("*old-content"));
}

// ---------------------------------------------------------------------------
// remove_block
// ---------------------------------------------------------------------------

#[test]
fn remove_block_should_remove_existing_block() {
    let content = "\
before
# >>> ufw-kit myblock
some content
# <<< ufw-kit myblock
after
";

    let result = remove_block(content, "myblock").unwrap();
    assert!(!result.contains(">>> ufw-kit myblock"));
    assert!(!result.contains("some content"));
    assert!(result.contains("before"));
    assert!(result.contains("after"));
}

#[test]
fn remove_block_should_return_unchanged_if_not_found() {
    let content = "unchanged content\n";
    let result = remove_block(content, "nonexistent").unwrap();
    assert_eq!(result, content);
}

// ---------------------------------------------------------------------------
// list_blocks
// ---------------------------------------------------------------------------

#[test]
fn list_blocks_should_find_all_blocks() {
    let content = "\
# >>> ufw-kit block1
content
# <<< ufw-kit block1
# >>> ufw-kit block2
content
# <<< ufw-kit block2
";

    let blocks = list_blocks(content);
    assert_eq!(blocks, vec!["block1", "block2"]);
}

#[test]
fn list_blocks_should_return_empty_for_no_blocks() {
    let content = "no managed blocks here\n";
    let blocks = list_blocks(content);
    assert!(blocks.is_empty());
}

// ---------------------------------------------------------------------------
// Edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn upsert_block_should_handle_empty_content() {
    let block = FrameworkRuleBlock {
        id: "test".into(),
        content: "content".into(),
        ipv6: false,
    };

    let result = upsert_block("", &block).unwrap();
    assert!(result.contains(">>> ufw-kit test"));
}

#[test]
fn remove_block_should_handle_content_without_trailing_newline() {
    let content = "before\n# >>> ufw-kit block\ncontent\n# <<< ufw-kit block";
    let result = remove_block(content, "block").unwrap();
    assert!(result.contains("before"));
    assert!(!result.contains(">>> ufw-kit block"));
}

// ---------------------------------------------------------------------------
// Weird edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn upsert_block_should_preserve_content_after_end_marker() {
    let content = "\
# >>> ufw-kit block
old
# <<< ufw-kit block
remaining content
";

    let block = FrameworkRuleBlock {
        id: "block".into(),
        content: "new".into(),
        ipv6: false,
    };

    let result = upsert_block(content, &block).unwrap();
    assert!(result.contains("new"));
    assert!(result.contains("remaining content"));
}

#[test]
fn list_blocks_should_handle_multiple_on_same_line() {
    // This shouldn't happen in practice but let's be robust
    let content = "# >>> ufw-kit a\n# >>> ufw-kit b\n";
    let blocks = list_blocks(content);
    assert_eq!(blocks.len(), 2);
}
