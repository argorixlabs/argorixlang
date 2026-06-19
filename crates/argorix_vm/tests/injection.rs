use argorix_vm::parse_injection;

#[test]
fn parses_explicit_four_part_injection() {
    let injection = parse_injection("User:ResearchAgent:tell:UserPrompt").unwrap();
    assert_eq!(injection.from, "User");
    assert_eq!(injection.to, "ResearchAgent");
    assert_eq!(injection.act, "tell");
    assert_eq!(injection.message_type, "UserPrompt");
}

#[test]
fn rejects_missing_extra_and_blank_injection_fields() {
    for invalid in [
        "User:Worker:tell",
        "User:Worker:tell:Ping:extra",
        "User::tell:Ping",
    ] {
        let error = parse_injection(invalid).unwrap_err();
        assert!(error.to_string().contains("invalid injection"));
    }
}
