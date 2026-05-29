use super::*;

#[test]
fn parse_simple_host_block() {
    let input = "\
Host example
    HostName example.com
    User alice
    IdentityFile ~/.ssh/id_ed25519

Host *
    ServerAliveInterval 60
";
    let ast = parse(input);
    assert_eq!(ast.nodes.len(), 3);

    match &ast.nodes[0] {
        ConfigNode::HostBlock { patterns, nodes, .. } => {
            assert_eq!(patterns, &["example"]);
            assert_eq!(nodes.len(), 3);
        }
        _ => panic!("expected HostBlock"),
    }
}

#[test]
fn round_trip_preserves_content() {
    let input = "\
# My SSH config
Host example
    HostName=example.com
    User alice

Host *
    ServerAliveInterval 60
";
    let ast = parse(input);
    let output = ast.to_string_lossless();
    assert_eq!(output, input);
}

#[test]
fn parse_equals_separator() {
    let input = "Host=myserver\n    HostName=192.168.1.1\n";
    let ast = parse(input);
    match &ast.nodes[0] {
        ConfigNode::HostBlock { header, .. } => {
            assert_eq!(header, "Host=myserver");
        }
        _ => panic!("expected HostBlock"),
    }
}

#[test]
fn split_trailing_comment_works() {
    let (val, comment) = split_trailing_comment("hello # a comment");
    assert_eq!(val, "hello");
    assert_eq!(comment, Some("a comment".to_owned()));

    let (val, comment) = split_trailing_comment("no comment");
    assert_eq!(val, "no comment");
    assert_eq!(comment, None);

    let (val, comment) = split_trailing_comment("\"path # here\" # real comment");
    assert_eq!(val, "\"path # here\"");
    assert_eq!(comment, Some("real comment".to_owned()));
}

#[test]
fn parse_directive_with_equals_in_value() {
    let (keyword, sep, rest) = parse_directive_parts("SetEnv FOO=bar");
    assert_eq!(keyword, "SetEnv");
    assert_eq!(sep, Separator::Space);
    assert_eq!(rest, "FOO=bar");
}

#[test]
fn parse_directive_equals_separator() {
    let (keyword, sep, rest) = parse_directive_parts("HostName=example.com");
    assert_eq!(keyword, "HostName");
    assert_eq!(sep, Separator::Equals);
    assert_eq!(rest, "example.com");
}

#[test]
fn round_trip_preserves_tab_indentation() {
    let input = "Host example\n\tHostName example.com\n\tUser alice\n";
    let ast = parse(input);
    let output = ast.to_string_lossless();
    assert_eq!(output, input);
}

#[test]
fn round_trip_with_comment_in_block() {
    let input = "Host example\n    # inline comment\n    HostName example.com\n";
    let ast = parse(input);
    let output = ast.to_string_lossless();
    assert_eq!(output, input);
}

// ---------------------------------------------------------------------------
// Edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_input() {
    let ast = parse("");
    assert!(ast.nodes.is_empty());
}

#[test]
fn parse_only_whitespace() {
    let ast = parse("   \n  \n");
    // Whitespace-only lines are treated as blank lines
    assert!(ast.nodes.iter().all(|n| matches!(n, ConfigNode::BlankLine)));
}

#[test]
fn parse_only_comments() {
    let input = "# comment 1\n# comment 2\n";
    let ast = parse(input);
    assert_eq!(ast.nodes.len(), 2);
    assert!(ast.nodes.iter().all(|n| matches!(n, ConfigNode::Comment { .. })));
}

#[test]
fn parse_host_with_no_directives() {
    let input = "Host empty\n";
    let ast = parse(input);
    assert_eq!(ast.nodes.len(), 1);
    match &ast.nodes[0] {
        ConfigNode::HostBlock { patterns, nodes, .. } => {
            assert_eq!(patterns, &["empty"]);
            assert!(nodes.is_empty());
        }
        _ => panic!("expected HostBlock"),
    }
}

#[test]
fn parse_host_with_multiple_patterns() {
    let input = "Host web1 web2 *.example.com\n    User admin\n";
    let ast = parse(input);
    match &ast.nodes[0] {
        ConfigNode::HostBlock { patterns, .. } => {
            assert_eq!(patterns, &["web1", "web2", "*.example.com"]);
        }
        _ => panic!("expected HostBlock"),
    }
}

#[test]
fn parse_directive_keyword_only() {
    let (keyword, sep, rest) = parse_directive_parts("ForwardAgent");
    assert_eq!(keyword, "ForwardAgent");
    assert_eq!(sep, Separator::Space);
    assert_eq!(rest, "");
}

#[test]
fn parse_directive_with_tab_separator() {
    let input = "Host example\n\tHostName example.com\n";
    let ast = parse(input);
    match &ast.nodes[0] {
        ConfigNode::HostBlock { nodes, .. } => {
            match &nodes[0] {
                ConfigNode::Directive { keyword, indent, .. } => {
                    assert_eq!(keyword, "HostName");
                    assert_eq!(indent, "\t");
                }
                _ => panic!("expected Directive"),
            }
        }
        _ => panic!("expected HostBlock"),
    }
}

#[test]
fn parse_multiple_host_blocks() {
    let input = "\
Host web
    HostName web.example.com

Host db
    HostName db.example.com
";
    let ast = parse(input);
    assert_eq!(ast.nodes.len(), 3); // web, blank, db
    match &ast.nodes[0] {
        ConfigNode::HostBlock { patterns, .. } => assert_eq!(patterns, &["web"]),
        _ => panic!("expected HostBlock"),
    }
    match &ast.nodes[2] {
        ConfigNode::HostBlock { patterns, .. } => assert_eq!(patterns, &["db"]),
        _ => panic!("expected HostBlock"),
    }
}

#[test]
fn parse_match_block() {
    let input = "Match host web*\n    User admin\n";
    let ast = parse(input);
    assert_eq!(ast.nodes.len(), 1);
    match &ast.nodes[0] {
        ConfigNode::MatchBlock { criteria, nodes, .. } => {
            assert_eq!(criteria, "host web*");
            assert_eq!(nodes.len(), 1);
        }
        _ => panic!("expected MatchBlock"),
    }
}

#[test]
fn round_trip_empty_input() {
    let ast = parse("");
    assert_eq!(ast.to_string_lossless(), "");
}

#[test]
fn line_indent_empty_string() {
    assert_eq!(line_indent(""), "");
}

#[test]
fn line_indent_no_indent() {
    assert_eq!(line_indent("hello"), "");
}

#[test]
fn line_indent_spaces() {
    assert_eq!(line_indent("    hello"), "    ");
}

#[test]
fn line_indent_tabs() {
    assert_eq!(line_indent("\thello"), "\t");
}

#[test]
fn line_indent_mixed() {
    assert_eq!(line_indent("  \t  hello"), "  \t  ");
}
