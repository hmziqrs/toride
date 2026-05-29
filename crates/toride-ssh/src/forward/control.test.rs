use super::*;

#[test]
fn parse_local_forward_line() {
    let line = "127.0.0.1 port 8080, forwarding to 10.0.0.1 port 80";
    let fwd = parse_forward_line(line, ForwardType::Local).unwrap();
    assert_eq!(fwd.local_addr, "127.0.0.1");
    assert_eq!(fwd.local_port, 8080);
    assert_eq!(fwd.remote_addr, "10.0.0.1");
    assert_eq!(fwd.remote_port, 80);
    assert_eq!(fwd.forward_type, ForwardType::Local);
}

#[test]
fn parse_local_forward_truncated_addr() {
    let line = "127.0.0. port 8080, forwarding to 10.0.0.1 port 80";
    let fwd = parse_forward_line(line, ForwardType::Local).unwrap();
    assert_eq!(fwd.local_addr, "127.0.0");
    assert_eq!(fwd.local_port, 8080);
    assert_eq!(fwd.remote_addr, "10.0.0.1");
    assert_eq!(fwd.remote_port, 80);
}

#[test]
fn parse_gateway_ports_forward() {
    let line = "* port 9090, forwarding to 192.168.1.1 port 443";
    let fwd = parse_forward_line(line, ForwardType::Local).unwrap();
    assert_eq!(fwd.local_addr, "*");
    assert_eq!(fwd.local_port, 9090);
    assert_eq!(fwd.remote_addr, "192.168.1.1");
    assert_eq!(fwd.remote_port, 443);
}

#[test]
fn parse_dynamic_forward_line() {
    let line = "127.0.0.1 port 1080";
    let fwd = parse_forward_line(line, ForwardType::Dynamic).unwrap();
    assert_eq!(fwd.local_addr, "127.0.0.1");
    assert_eq!(fwd.local_port, 1080);
    assert_eq!(fwd.forward_type, ForwardType::Dynamic);
}

#[test]
fn parse_dynamic_forward_gateway() {
    let line = "* port 1080";
    let fwd = parse_forward_line(line, ForwardType::Dynamic).unwrap();
    assert_eq!(fwd.local_addr, "*");
    assert_eq!(fwd.local_port, 1080);
}

#[test]
fn parse_full_output() {
    let output = "\
Local connections:
  127.0.0.1 port 8080, forwarding to 10.0.0.1 port 80
  0.0.0.0 port 9090, forwarding to 192.168.1.1 port 443
Remote connections:
  127.0.0.1 port 2222, forwarding to 127.0.0.1 port 22
Dynamic connections:
  127.0.0.1 port 1080
";
    let fwds = parse_forward_output(output);
    assert_eq!(fwds.len(), 4);
    assert_eq!(fwds[0].forward_type, ForwardType::Local);
    assert_eq!(fwds[0].local_port, 8080);
    assert_eq!(fwds[1].forward_type, ForwardType::Local);
    assert_eq!(fwds[1].local_port, 9090);
    assert_eq!(fwds[2].forward_type, ForwardType::Remote);
    assert_eq!(fwds[2].remote_port, 22);
    assert_eq!(fwds[3].forward_type, ForwardType::Dynamic);
    assert_eq!(fwds[3].local_port, 1080);
}

#[test]
fn parse_empty_sections() {
    let output = "\
Local connections:
Remote connections:
Dynamic connections:
";
    let fwds = parse_forward_output(output);
    assert!(fwds.is_empty());
}

#[test]
fn parse_output_with_no_forwards() {
    let output = "";
    let fwds = parse_forward_output(output);
    assert!(fwds.is_empty());
}

#[test]
fn parse_output_with_error_message() {
    let output = "No forwards.\nLocal connections:\n";
    let fwds = parse_forward_output(output);
    assert!(fwds.is_empty());
}

#[test]
fn parse_remote_forward_line() {
    let line = "0.0.0.0 port 2222, forwarding to 127.0.0.1 port 22";
    let fwd = parse_forward_line(line, ForwardType::Remote).unwrap();
    assert_eq!(fwd.local_addr, "0.0.0.0");
    assert_eq!(fwd.local_port, 2222);
    assert_eq!(fwd.remote_addr, "127.0.0.1");
    assert_eq!(fwd.remote_port, 22);
    assert_eq!(fwd.forward_type, ForwardType::Remote);
}

#[test]
fn extract_host_various_patterns() {
    assert_eq!(
        extract_host_from_name("cm-deploy@web01.example.com:22"),
        "web01.example.com"
    );
    assert_eq!(extract_host_from_name("control-root@db:5432"), "db");
    assert_eq!(extract_host_from_name("mux-user@bastion:22"), "bastion");
    assert_eq!(extract_host_from_name("ctrl-user@jump:22"), "jump");
    assert_eq!(
        extract_host_from_name("ssh-abc123def456-12345"),
        "abc123def456-12345"
    );
}

#[test]
fn extract_pid_from_patterns() {
    assert_eq!(extract_pid_from_name("ssh-abc123-48291"), Some(48291));
    assert_eq!(extract_pid_from_name("cm-user@host:22"), None);
    assert_eq!(extract_pid_from_name("ssh-hash-0"), None);
}

#[test]
fn cancel_spec_local_forward() {
    let fwd = PortForward {
        local_addr: "127.0.0.1".to_owned(),
        local_port: 8080,
        remote_addr: "10.0.0.1".to_owned(),
        remote_port: 80,
        forward_type: ForwardType::Local,
    };
    let spec = if fwd.forward_type == ForwardType::Dynamic {
        format!("[{}]:{}", fwd.local_addr, fwd.local_port)
    } else {
        format!(
            "[{}]:{}:{}:{}",
            fwd.local_addr, fwd.local_port, fwd.remote_addr, fwd.remote_port
        )
    };
    assert_eq!(spec, "[127.0.0.1]:8080:10.0.0.1:80");
}

#[test]
fn cancel_spec_dynamic_forward() {
    let fwd = PortForward {
        local_addr: "127.0.0.1".to_owned(),
        local_port: 1080,
        remote_addr: String::new(),
        remote_port: 0,
        forward_type: ForwardType::Dynamic,
    };
    let spec = if fwd.forward_type == ForwardType::Dynamic {
        format!("[{}]:{}", fwd.local_addr, fwd.local_port)
    } else {
        unreachable!()
    };
    assert_eq!(spec, "[127.0.0.1]:1080");
}
